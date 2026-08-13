use std::{fs, io, path::Path, time::UNIX_EPOCH};

use crate::inventory::InventoryEntryKind;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalEntryMetadata {
    pub byte_size: u64,
    pub modified_unix_seconds: Option<i64>,
    pub kind: InventoryEntryKind,
}

pub fn verify_directory(root: &Path) -> io::Result<()> {
    match fs::metadata(root) {
        Ok(metadata) if metadata.is_dir() => Ok(()),
        Ok(_) => Err(io::Error::new(
            io::ErrorKind::NotADirectory,
            "not a directory",
        )),
        Err(error) => Err(error),
    }
}

pub fn native_metadata(path: &Path) -> io::Result<LocalEntryMetadata> {
    let metadata = fs::symlink_metadata(path)?;
    Ok(LocalEntryMetadata {
        byte_size: metadata.len(),
        modified_unix_seconds: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| duration.as_secs().try_into().ok()),
        kind: if metadata.is_dir() {
            InventoryEntryKind::Directory
        } else {
            InventoryEntryKind::File
        },
    })
}
