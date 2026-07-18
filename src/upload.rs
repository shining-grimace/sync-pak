use std::error::Error;
use std::fmt;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::provider_capabilities::{ObjectWriteMetadata, ObjectWriter, ProviderError};

pub async fn upload_from_path<T: ObjectWriter>(
    provider: &T,
    bucket: &str,
    key: &str,
    source: &Path,
) -> Result<(), UploadError> {
    let metadata = std::fs::metadata(source).map_err(UploadError::Local)?;
    let contents = std::fs::read(source).map_err(UploadError::Local)?;
    let write_metadata = ObjectWriteMetadata {
        source_modified_unix_seconds: metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .and_then(|duration| duration.as_secs().try_into().ok()),
    };
    provider
        .write_with_metadata(bucket, key, &contents, &write_metadata)
        .await
        .map_err(UploadError::Provider)
}

#[derive(Debug)]
pub enum UploadError {
    Provider(ProviderError),
    Local(std::io::Error),
}

impl fmt::Display for UploadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Provider(error) => error.fmt(formatter),
            Self::Local(error) => write!(formatter, "could not read the upload source: {error}"),
        }
    }
}

impl Error for UploadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Provider(error) => Some(error),
            Self::Local(error) => Some(error),
        }
    }
}

#[cfg(test)]
#[path = "upload_tests.rs"]
mod tests;
