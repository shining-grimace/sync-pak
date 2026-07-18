use std::error::Error;
use std::fmt;
use std::path::Path;

use crate::atomic_write::atomic_write;
use crate::provider_capabilities::{ObjectReader, ProviderError};

pub async fn download_to_path<T: ObjectReader>(
    provider: &T,
    bucket: &str,
    key: &str,
    destination: &Path,
) -> Result<(), DownloadError> {
    let contents = provider
        .read(bucket, key)
        .await
        .map_err(DownloadError::Provider)?;
    atomic_write(destination, &contents).map_err(DownloadError::Local)
}

#[derive(Debug)]
pub enum DownloadError {
    Provider(ProviderError),
    Local(std::io::Error),
}

impl fmt::Display for DownloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Provider(error) => error.fmt(formatter),
            Self::Local(error) => write!(formatter, "could not save the downloaded file: {error}"),
        }
    }
}

impl Error for DownloadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Provider(error) => Some(error),
            Self::Local(error) => Some(error),
        }
    }
}

#[cfg(test)]
#[path = "download_tests.rs"]
mod tests;
