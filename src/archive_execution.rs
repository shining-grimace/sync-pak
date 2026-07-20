use std::{error::Error, fmt, path::Path};

use crate::{
    archive_create::{ArchiveCreateError, stage_archive},
    archive_naming::{ArchiveNameError, archive_filename},
    archive_upload::{ArchiveUploadError, ArchiveUploader, upload_staged_archive},
    cancellation::CancellationToken,
    inventory::{Inventory, InventoryError, RelativePath},
    transfer_paths::LocalTransferRoot,
};

/// Creates and uploads a timestamped ZIP archive, retaining its staging file on upload failure.
pub async fn create_and_upload_archive<U: ArchiveUploader>(
    source_root: &LocalTransferRoot,
    inventory: &Inventory,
    staging_directory: &Path,
    timestamp: &str,
    connection_name: &str,
    uploader: &U,
    cancellation: &CancellationToken,
    jitter_seed: u64,
) -> Result<String, ArchiveExecutionError<U::Error>> {
    if cancellation.check().is_err() {
        return Err(ArchiveExecutionError::Cancelled);
    }
    let filename =
        archive_filename(timestamp, connection_name).map_err(ArchiveExecutionError::Name)?;
    let destination = RelativePath::new(filename.clone()).map_err(ArchiveExecutionError::Path)?;
    let staged = stage_archive(source_root, inventory, staging_directory, filename.as_ref())
        .map_err(ArchiveExecutionError::Create)?;
    upload_staged_archive(uploader, staged, &destination, cancellation, jitter_seed)
        .await
        .map_err(ArchiveExecutionError::Upload)?;
    Ok(filename)
}

#[derive(Debug)]
pub enum ArchiveExecutionError<E> {
    Cancelled,
    Name(ArchiveNameError),
    Path(InventoryError),
    Create(ArchiveCreateError),
    Upload(ArchiveUploadError<E>),
}

impl<E: fmt::Display> fmt::Display for ArchiveExecutionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("archive creation was cancelled"),
            Self::Name(error) => error.fmt(formatter),
            Self::Path(error) => error.fmt(formatter),
            Self::Create(error) => error.fmt(formatter),
            Self::Upload(error) => error.fmt(formatter),
        }
    }
}

impl<E: Error + 'static> Error for ArchiveExecutionError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Name(error) => Some(error),
            Self::Path(error) => Some(error),
            Self::Create(error) => Some(error),
            Self::Upload(error) => Some(error),
            Self::Cancelled => None,
        }
    }
}

#[cfg(test)]
#[path = "archive_execution_tests.rs"]
mod tests;
