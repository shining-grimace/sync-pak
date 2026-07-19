use std::{error::Error, fmt, fs, io::Write, path::Path};

use zip::{CompressionMethod, ZipWriter, write::SimpleFileOptions};

use crate::{
    inventory::{Inventory, InventoryEntryKind},
    transfer_paths::LocalTransferRoot,
};

/// Writes an inventory as a ZIP, then publishes it without replacing an existing archive.
pub fn create_archive(
    source_root: &LocalTransferRoot,
    inventory: &Inventory,
    destination: &Path,
) -> Result<(), ArchiveCreateError> {
    if destination.exists() {
        return Err(ArchiveCreateError::Collision);
    }
    let directory = destination
        .parent()
        .ok_or(ArchiveCreateError::InvalidDestination)?;
    let filename = destination
        .file_name()
        .ok_or(ArchiveCreateError::InvalidDestination)?;
    fs::create_dir_all(directory).map_err(ArchiveCreateError::Local)?;
    let temporary = directory.join(format!(
        ".{}-{}.tmp",
        filename.to_string_lossy(),
        uuid::Uuid::new_v4()
    ));
    let result = write_archive(source_root, inventory, &temporary)
        .and_then(|()| fs::hard_link(&temporary, destination).map_err(ArchiveCreateError::Local));
    let _ = fs::remove_file(&temporary);
    result
}

fn write_archive(
    source_root: &LocalTransferRoot,
    inventory: &Inventory,
    temporary: &Path,
) -> Result<(), ArchiveCreateError> {
    let file = fs::File::options()
        .create_new(true)
        .write(true)
        .open(temporary)
        .map_err(ArchiveCreateError::Local)?;
    let mut archive = ZipWriter::new(file);
    for entry in inventory.entries() {
        let name = entry.path.as_str();
        match &entry.kind {
            InventoryEntryKind::Directory => archive
                .add_directory(format!("{name}/"), options(0o40755))
                .map_err(ArchiveCreateError::Zip)?,
            InventoryEntryKind::File => {
                archive
                    .start_file(name, options(0o100644))
                    .map_err(ArchiveCreateError::Zip)?;
                let mut source = fs::File::open(source_root.resolve(&entry.path))
                    .map_err(ArchiveCreateError::Local)?;
                std::io::copy(&mut source, &mut archive).map_err(ArchiveCreateError::Local)?;
            }
            InventoryEntryKind::Symlink { target } => {
                archive
                    .start_file(name, options(0o120777))
                    .map_err(ArchiveCreateError::Zip)?;
                archive
                    .write_all(target.as_bytes())
                    .map_err(ArchiveCreateError::Local)?;
            }
        }
    }
    archive
        .finish()
        .map_err(ArchiveCreateError::Zip)?
        .sync_all()
        .map_err(ArchiveCreateError::Local)
}

fn options(permissions: u32) -> SimpleFileOptions {
    SimpleFileOptions::default()
        .compression_method(CompressionMethod::Stored)
        .unix_permissions(permissions)
}

#[derive(Debug)]
pub enum ArchiveCreateError {
    Collision,
    InvalidDestination,
    Local(std::io::Error),
    Zip(zip::result::ZipError),
}

impl fmt::Display for ArchiveCreateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Collision => formatter.write_str("an archive already exists at this filename"),
            Self::InvalidDestination => {
                formatter.write_str("archive destination must include a filename")
            }
            Self::Local(error) => write!(formatter, "could not create archive: {error}"),
            Self::Zip(error) => write!(formatter, "could not write ZIP archive: {error}"),
        }
    }
}

impl Error for ArchiveCreateError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Local(error) => Some(error),
            Self::Zip(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "archive_create_tests.rs"]
mod tests;
