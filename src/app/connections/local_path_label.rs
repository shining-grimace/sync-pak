/// Formats local paths without exposing opaque document-provider identifiers.
pub(super) fn label(path: &str) -> String {
    let Some(document_id) =
        path.strip_prefix("content://com.android.externalstorage.documents/tree/")
    else {
        return if path.starts_with("content://") {
            "(Document storage)/Selected folder".into()
        } else {
            path.into()
        };
    };
    let Some(decoded) = percent_decode(document_id) else {
        return path.into();
    };
    let (volume, folder) = decoded.split_once(':').unwrap_or((&decoded, ""));
    if volume.is_empty() {
        return path.into();
    }
    if folder.is_empty() {
        format!("({volume})")
    } else {
        format!("({volume})/{folder}")
    }
}

fn percent_decode(value: &str) -> Option<String> {
    let bytes = value.as_bytes();
    let mut decoded = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        if bytes[index] == b'%' {
            let pair = std::str::from_utf8(bytes.get(index + 1..index + 3)?).ok()?;
            decoded.push(u8::from_str_radix(pair, 16).ok()?);
            index += 3;
        } else {
            decoded.push(bytes[index]);
            index += 1;
        }
    }
    String::from_utf8(decoded).ok()
}

#[cfg(test)]
mod tests {
    use super::label;

    #[test]
    fn formats_storage_roots_and_nested_folders() {
        let prefix = "content://com.android.externalstorage.documents/tree/";
        for (id, expected) in [
            ("primary%3A", "(primary)"),
            (
                "primary%3ADocuments%2FMy%20Folder",
                "(primary)/Documents/My Folder",
            ),
            ("1234-ABCD%3aPhotos%2f2026", "(1234-ABCD)/Photos/2026"),
            (
                "primary%3ACaf%C3%A9%2F100%25%20%2B",
                "(primary)/Café/100% +",
            ),
        ] {
            assert_eq!(label(&format!("{prefix}{id}")), expected);
        }
    }

    #[test]
    fn opaque_provider_ids_are_never_used_as_folder_names() {
        for uri in [
            "content://com.google.apps.docs.storage/tree/acc%3D8%3Bdoc%3Dencoded%3D3Pa8cy",
            "content://another.provider/tree/private-token",
        ] {
            assert_eq!(label(uri), "(Document storage)/Selected folder");
        }
    }

    #[test]
    fn preserves_native_paths_and_malformed_storage_uris() {
        for path in [
            "/home/user/My%20Folder",
            r"C:\Users\Documents",
            "content://com.android.externalstorage.documents/tree/primary%3AFolder%",
            "content://com.android.externalstorage.documents/tree/primary%3A%FF",
            "content://com.android.externalstorage.documents/tree/%3AFolder",
        ] {
            assert_eq!(label(path), path);
        }
    }
}
