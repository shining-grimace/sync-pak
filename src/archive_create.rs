use std::{
    error::Error,
    fmt, fs,
    io::Write,
    path::{Path, PathBuf},
};

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
    let staged = stage_archive(source_root, inventory, directory, filename)?;
    let result = fs::hard_link(staged.path(), destination).map_err(ArchiveCreateError::Local);
    let _ = staged.discard();
    result
}

/// Creates a ZIP in an unobservable local temporary file for later storage or publication.
///
/// A failed remote upload deliberately leaves this file in place, so callers must discard it
/// only after the provider has confirmed successful storage.
pub fn stage_archive(
    source_root: &LocalTransferRoot,
    inventory: &Inventory,
    staging_directory: &Path,
    filename: &std::ffi::OsStr,
) -> Result<StagedArchive, ArchiveCreateError> {
    fs::create_dir_all(staging_directory).map_err(ArchiveCreateError::Local)?;
    let path = staging_directory.join(format!(
        ".{}-{}.tmp",
        filename.to_string_lossy(),
        uuid::Uuid::new_v4()
    ));
    write_archive(source_root, inventory, &path)?;
    Ok(StagedArchive { path })
}

/// A complete ZIP that is still local and has not yet been published or uploaded.
#[derive(Debug)]
pub struct StagedArchive {
    path: PathBuf,
}

impl StagedArchive {
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Removes this temporary archive after confirmed storage at its destination.
    pub fn discard(&self) -> std::io::Result<()> {
        fs::remove_file(&self.path)
    }
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
