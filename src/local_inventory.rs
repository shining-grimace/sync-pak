use std::error::Error;
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::time::UNIX_EPOCH;

use crate::inventory::{
    Inventory, InventoryEntry, InventoryEntryKind, InventoryError, RelativePath,
};

pub trait LocalInventoryAccess {
    fn inventory(&self, root: &Path) -> Result<Inventory, LocalInventoryError>;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct NativeLocalInventory;

impl LocalInventoryAccess for NativeLocalInventory {
    fn inventory(&self, root: &Path) -> Result<Inventory, LocalInventoryError> {
        let mut entries = Vec::new();
        collect_directory(root, root, &mut entries)?;
        Inventory::new(entries).map_err(LocalInventoryError::InvalidInventory)
    }
}

fn collect_directory(
    root: &Path,
    directory: &Path,
    entries: &mut Vec<InventoryEntry>,
) -> Result<(), LocalInventoryError> {
    let children = fs::read_dir(directory).map_err(|source| LocalInventoryError::Io {
        operation: "read directory",
        path: directory.into(),
        source,
    })?;
    for child in children {
        let child = child.map_err(|source| LocalInventoryError::Io {
            operation: "read directory entry",
            path: directory.into(),
            source,
        })?;
        let path = child.path();
        let metadata = fs::symlink_metadata(&path).map_err(|source| LocalInventoryError::Io {
            operation: "read metadata",
            path: path.clone(),
            source,
        })?;
        let relative_path = relative_path(root, &path)?;
        let kind = if metadata.file_type().is_symlink() {
            let target = fs::read_link(&path).map_err(|source| LocalInventoryError::Io {
                operation: "read symbolic link",
                path: path.clone(),
                source,
            })?;
            InventoryEntryKind::Symlink {
                target: utf8_path(&target)?,
            }
        } else if metadata.is_dir() {
            InventoryEntryKind::Directory
        } else if metadata.is_file() {
            InventoryEntryKind::File
        } else {
            return Err(LocalInventoryError::UnsupportedFileType(path));
        };
        entries.push(InventoryEntry::new(
            relative_path,
            kind.clone(),
            metadata.len(),
            modified_seconds(&metadata),
        ));
        if kind == InventoryEntryKind::Directory {
            collect_directory(root, &path, entries)?;
        }
    }
    Ok(())
}

fn relative_path(root: &Path, path: &Path) -> Result<RelativePath, LocalInventoryError> {
    let relative = path
        .strip_prefix(root)
        .expect("walked paths are below root");
    let components = relative
        .components()
        .map(|component| utf8_path(Path::new(component.as_os_str())))
        .collect::<Result<Vec<_>, _>>()?;
    RelativePath::new(components.join("/")).map_err(LocalInventoryError::InvalidInventory)
}

fn utf8_path(path: &Path) -> Result<String, LocalInventoryError> {
    path.to_str()
        .map(str::to_owned)
        .ok_or_else(|| LocalInventoryError::NonUtf8Path(path.into()))
}

fn modified_seconds(metadata: &fs::Metadata) -> Option<i64> {
    metadata
        .modified()
        .ok()?
        .duration_since(UNIX_EPOCH)
        .ok()?
        .as_secs()
        .try_into()
        .ok()
}

#[derive(Debug)]
pub enum LocalInventoryError {
    Io {
        operation: &'static str,
        path: PathBuf,
        source: io::Error,
    },
    NonUtf8Path(PathBuf),
    UnsupportedFileType(PathBuf),
    InvalidInventory(InventoryError),
}

impl fmt::Display for LocalInventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io {
                operation,
                path,
                source,
            } => write!(
                formatter,
                "could not {operation} at {}: {source}",
                path.display()
            ),
            Self::NonUtf8Path(path) => {
                write!(formatter, "path is not valid UTF-8: {}", path.display())
            }
            Self::UnsupportedFileType(path) => {
                write!(
                    formatter,
                    "unsupported filesystem entry: {}",
                    path.display()
                )
            }
            Self::InvalidInventory(source) => source.fmt(formatter),
        }
    }
}

impl Error for LocalInventoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Io { source, .. } => Some(source),
            Self::InvalidInventory(source) => Some(source),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "local_inventory_tests.rs"]
mod tests;
