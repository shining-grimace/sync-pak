use std::{error::Error, fmt, path::Path};

use crate::{
    inventory::{Inventory, InventoryError, RelativePath},
    operations::archive::create::{ArchiveCreateError, stage_archive_with_cancellation},
    operations::archive::naming::{ArchiveNameError, archive_filename},
    operations::archive::upload::{ArchiveUploadError, ArchiveUploader, upload_staged_archive},
    operations::cancellation::CancellationToken,
    operations::transfer::paths::LocalTransferRoot,
};

/// Local input and naming settings for one archive.
#[derive(Clone, Copy)]
pub struct ArchiveSource<'a> {
    pub source_root: &'a LocalTransferRoot,
    pub inventory: &'a Inventory,
    pub staging_directory: &'a Path,
    pub timestamp: &'a str,
    pub connection_name: &'a str,
}

/// Creates and uploads a timestamped ZIP archive, retaining its staging file on upload failure.
pub async fn create_and_upload_archive<U: ArchiveUploader>(
    source: ArchiveSource<'_>,
    uploader: &U,
    cancellation: &CancellationToken,
    jitter_seed: u64,
) -> Result<String, ArchiveExecutionError<U::Error>> {
    let ArchiveSource {
        source_root,
        inventory,
        staging_directory,
        timestamp,
        connection_name,
    } = source;
    if cancellation.check().is_err() {
        return Err(ArchiveExecutionError::Cancelled);
    }
    let filename =
        archive_filename(timestamp, connection_name).map_err(ArchiveExecutionError::Name)?;
    let destination = RelativePath::new(filename.clone()).map_err(ArchiveExecutionError::Path)?;
    let staged = stage_archive_with_cancellation(
        source_root,
        inventory,
        staging_directory,
        filename.as_ref(),
        cancellation,
    )
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
mod tests;
