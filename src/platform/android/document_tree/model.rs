use std::io;

use serde::Deserialize;

use crate::inventory::{InventoryEntry, InventoryEntryKind, RelativePath};

pub fn status(value: i32) -> io::Result<()> {
    if value == 0 {
        Ok(())
    } else {
        Err(status_error(value))
    }
}

pub fn status_error(value: i32) -> io::Error {
    let kind = match value {
        -2 => io::ErrorKind::NotFound,
        -3 => io::ErrorKind::NotADirectory,
        -4 => io::ErrorKind::IsADirectory,
        -5 => io::ErrorKind::PermissionDenied,
        -6 => io::ErrorKind::InvalidInput,
        _ => io::ErrorKind::Other,
    };
    io::Error::new(kind, "Android document provider operation failed")
}

pub fn unavailable() -> io::Error {
    io::Error::other("Android document provider is unavailable")
}

#[derive(Deserialize)]
pub struct DocumentMetadata {
    kind: String,
    pub size: u64,
    pub modified: Option<i64>,
}

impl DocumentMetadata {
    pub fn is_directory(&self) -> bool {
        self.kind == "directory"
    }
}

#[derive(Deserialize)]
pub struct DocumentEntry {
    path: String,
    kind: String,
    size: u64,
    modified: Option<i64>,
}

impl TryFrom<DocumentEntry> for InventoryEntry {
    type Error = io::Error;

    fn try_from(entry: DocumentEntry) -> Result<Self, Self::Error> {
        let path = RelativePath::new(entry.path)
            .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid document path"))?;
        let kind = match entry.kind.as_str() {
            "file" => InventoryEntryKind::File,
            "directory" => InventoryEntryKind::Directory,
            _ => {
                return Err(io::Error::new(
                    io::ErrorKind::InvalidData,
                    "invalid document type",
                ));
            }
        };
        Ok(InventoryEntry::new(path, kind, entry.size, entry.modified))
    }
}
