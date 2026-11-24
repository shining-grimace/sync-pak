use std::io::Read;

use crate::{
    configuration::{ConfigStore, ProviderRepository},
    inventory::{InventoryEntry, InventoryEntryKind},
    operations::{request::RunRequest, transfer::paths::LocalTransferRoot},
    platform::PlatformCredentialStore,
    providers::s3::transport::S3Transport,
};

use super::TEXT_LIMIT;

const IMAGE_LIMIT: u64 = 24_000_000;

#[derive(Clone, Copy)]
pub(super) enum Format {
    Text,
    Image,
}

pub(super) fn text_file(path: &str) -> bool {
    matches!(
        extension(path).as_str(),
        "txt"
            | "md"
            | "markdown"
            | "json"
            | "jsonl"
            | "csv"
            | "tsv"
            | "xml"
            | "html"
            | "css"
            | "yaml"
            | "yml"
            | "toml"
            | "ini"
            | "log"
            | "rs"
            | "py"
            | "js"
            | "ts"
            | "sh"
            | "c"
            | "h"
            | "cpp"
            | "java"
            | "go"
            | "sql"
            | "conf"
    )
}

fn extension(path: &str) -> String {
    std::path::Path::new(path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase()
}

pub(super) fn preview_type(entry: &InventoryEntry) -> Option<(Format, u64)> {
    if entry.kind != InventoryEntryKind::File {
        return None;
    }
    let path = entry.path.as_str();
    if text_file(path) && entry.byte_size < TEXT_LIMIT {
        return Some((Format::Text, TEXT_LIMIT - 1));
    }
    if entry.byte_size <= IMAGE_LIMIT
        && matches!(
            extension(path).as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "ico"
        )
    {
        return Some((Format::Image, IMAGE_LIMIT));
    }
    None
}

pub(super) enum Preview {
    Text(String),
    Image(image::RgbaImage),
}

fn decode(bytes: Vec<u8>, format: Format, limit: u64) -> Option<Preview> {
    if bytes.len() as u64 > limit {
        return None;
    }
    match format {
        Format::Text => String::from_utf8(bytes)
            .ok()
            .filter(|text| !text.contains('\0'))
            .map(Preview::Text),
        Format::Image => {
            let mut reader = image::ImageReader::new(std::io::Cursor::new(bytes))
                .with_guessed_format()
                .ok()?;
            let mut limits = image::Limits::default();
            limits.max_image_width = Some(8192);
            limits.max_image_height = Some(8192);
            limits.max_alloc = Some(128 * 1024 * 1024);
            reader.limits(limits);
            Some(Preview::Image(
                reader.decode().ok()?.thumbnail(2400, 2400).into_rgba8(),
            ))
        }
    }
}

pub(super) fn load_local(root: &LocalTransferRoot, entry: &InventoryEntry) -> Option<Preview> {
    let (format, limit) = preview_type(entry)?;
    let file = match root {
        LocalTransferRoot::FileSystem(base) => {
            let path = root.native_path(&entry.path)?;
            let resolved = path.canonicalize().ok()?;
            if !resolved.starts_with(base.canonicalize().ok()?)
                || !path.symlink_metadata().ok()?.is_file()
            {
                return None;
            }
            std::fs::File::open(resolved).ok()?
        }
        #[cfg(target_os = "android")]
        LocalTransferRoot::AndroidTree(_) => root.open_read(&entry.path).ok()?,
    };
    let mut bytes = Vec::new();
    file.take(limit + 1).read_to_end(&mut bytes).ok()?;
    decode(bytes, format, limit)
}

pub(super) fn load_remote(
    request: &RunRequest,
    prefix: &str,
    entry: &InventoryEntry,
) -> Option<Preview> {
    let (format, limit) = preview_type(entry)?;
    let configuration = ConfigStore::for_current_platform().ok()?;
    let store = PlatformCredentialStore::new().ok()?;
    let credentials = ProviderRepository::new(&configuration, &store)
        .load_credentials(&request.provider.id)
        .ok()?;
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .ok()?;
    let bytes = runtime.block_on(async {
        tokio::time::timeout(std::time::Duration::from_secs(30), async {
            let transport = S3Transport::connect_bucket(
                &request.provider,
                credentials,
                &request.connection.bucket,
                request.connection.transport.as_ref(),
            )
            .await
            .ok()?;
            transport
                .read_preview(
                    &request.connection.bucket,
                    &format!("{prefix}{}", entry.path.as_str()),
                    limit,
                )
                .await
        })
        .await
        .ok()?
    })?;
    decode(bytes, format, limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::inventory::RelativePath;

    #[test]
    fn previews_only_supported_files_within_their_size_limit() {
        let mut entry = InventoryEntry::new(
            RelativePath::new("Notes.JSON").unwrap(),
            InventoryEntryKind::File,
            TEXT_LIMIT - 1,
            None,
        );
        assert!(matches!(preview_type(&entry), Some((Format::Text, _))));
        entry.byte_size += 1;
        assert!(preview_type(&entry).is_none());

        entry.path = RelativePath::new("image.PNG").unwrap();
        assert!(matches!(preview_type(&entry), Some((Format::Image, _))));
        entry.kind = InventoryEntryKind::Directory;
        assert!(preview_type(&entry).is_none());

        entry.path = RelativePath::new("document.pdf").unwrap();
        entry.kind = InventoryEntryKind::File;
        assert!(preview_type(&entry).is_none());
    }

    #[test]
    fn checks_actual_content_size_and_text_encoding() {
        assert!(decode(b"hello".to_vec(), Format::Text, 4).is_none());
        assert!(decode(vec![0xff], Format::Text, 4).is_none());
        assert!(matches!(
            decode(b"hello".to_vec(), Format::Text, 5),
            Some(Preview::Text(text)) if text == "hello"
        ));
    }
}
