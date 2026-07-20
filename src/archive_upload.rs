use std::{error::Error, fmt, future::Future, path::Path};

use crate::{archive_create::StagedArchive, cancellation::CancellationToken};

/// Stores a complete, staged archive at its destination.
pub trait ArchiveUploader {
    type Error;

    fn upload(
        &self,
        source: &Path,
        destination: &str,
        cancellation: &CancellationToken,
    ) -> impl Future<Output = Result<(), Self::Error>>;
}

/// Uploads a staged archive and removes its local copy only after provider confirmation.
pub async fn upload_staged_archive<U: ArchiveUploader>(
    uploader: &U,
    staged: StagedArchive,
    destination: &str,
    cancellation: &CancellationToken,
) -> Result<(), ArchiveUploadError<U::Error>> {
    if cancellation.check().is_err() {
        return Err(ArchiveUploadError::Cancelled { staged });
    }
    if let Err(error) = uploader
        .upload(staged.path(), destination, cancellation)
        .await
    {
        return Err(ArchiveUploadError::Upload { staged, error });
    }
    if let Err(error) = staged.discard() {
        return Err(ArchiveUploadError::Cleanup { staged, error });
    }
    Ok(())
}

/// An archive upload that deliberately leaves its recoverable local ZIP available.
#[derive(Debug)]
pub enum ArchiveUploadError<E> {
    Cancelled {
        staged: StagedArchive,
    },
    Upload {
        staged: StagedArchive,
        error: E,
    },
    Cleanup {
        staged: StagedArchive,
        error: std::io::Error,
    },
}

impl<E: fmt::Display> fmt::Display for ArchiveUploadError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled { .. } => {
                formatter.write_str("archive upload was cancelled; the temporary archive was kept")
            }
            Self::Upload { error, .. } => write!(
                formatter,
                "archive upload failed; the temporary archive was kept: {error}"
            ),
            Self::Cleanup { error, .. } => write!(
                formatter,
                "archive was uploaded, but its temporary local copy could not be removed: {error}"
            ),
        }
    }
}

impl<E: Error + 'static> Error for ArchiveUploadError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Upload { error, .. } => Some(error),
            Self::Cleanup { error, .. } => Some(error),
            Self::Cancelled { .. } => None,
        }
    }
}

#[cfg(test)]
#[path = "archive_upload_tests.rs"]
mod tests;
