use std::{
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
};

use crate::{
    cancellation::CancellationToken, inventory::Inventory, transfer_paths::LocalTransferRoot,
};

use crate::archive_create_writer::write_archive;

/// Writes an inventory as a ZIP, then publishes it without replacing an existing archive.
pub fn create_archive(
    source_root: &LocalTransferRoot,
    inventory: &Inventory,
    destination: &Path,
) -> Result<(), ArchiveCreateError> {
    create_archive_with_cancellation(
        source_root,
        inventory,
        destination,
        &CancellationToken::default(),
    )
}

/// Publishes an archive only if ZIP creation completes without cancellation.
pub fn create_archive_with_cancellation(
    source_root: &LocalTransferRoot,
    inventory: &Inventory,
    destination: &Path,
    cancellation: &CancellationToken,
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
    let staged =
        stage_archive_with_cancellation(source_root, inventory, directory, filename, cancellation)?;
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
    stage_archive_with_cancellation(
        source_root,
        inventory,
        staging_directory,
        filename,
        &CancellationToken::default(),
    )
}

/// Creates a ZIP in a temporary file while checking cancellation between file chunks.
pub fn stage_archive_with_cancellation(
    source_root: &LocalTransferRoot,
    inventory: &Inventory,
    staging_directory: &Path,
    filename: &std::ffi::OsStr,
    cancellation: &CancellationToken,
) -> Result<StagedArchive, ArchiveCreateError> {
    fs::create_dir_all(staging_directory).map_err(ArchiveCreateError::Local)?;
    let path = staging_directory.join(format!(
        ".{}.sync-pak-{}.tmp",
        filename.to_string_lossy(),
        uuid::Uuid::new_v4()
    ));
    if let Err(error) = write_archive(source_root, inventory, &path, cancellation) {
        let _ = fs::remove_file(&path);
        return Err(error);
    }
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

#[derive(Debug)]
pub enum ArchiveCreateError {
    Cancelled,
    Collision,
    InvalidDestination,
    Local(std::io::Error),
    Zip(zip::result::ZipError),
}

impl fmt::Display for ArchiveCreateError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("archive creation was cancelled"),
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
