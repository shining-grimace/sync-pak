use std::error::Error;
use std::fmt;
use std::path::Path;

use crate::atomic_write::atomic_write;
use crate::provider_capabilities::{ObjectReader, ProviderError};
use crate::retry::{RetryPolicy, RetrySleeper};

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

pub async fn download_to_path_with_retry<T: ObjectReader, S: RetrySleeper>(
    provider: &T,
    bucket: &str,
    key: &str,
    destination: &Path,
    policy: &RetryPolicy,
    sleeper: &S,
    jitter_seed: u64,
) -> Result<(), DownloadError> {
    let mut completed_attempts = 0;
    loop {
        completed_attempts += 1;
        match provider.read(bucket, key).await {
            Ok(contents) => {
                return atomic_write(destination, &contents).map_err(DownloadError::Local);
            }
            Err(error) => {
                match policy.delay_after_failure(completed_attempts, error, None, jitter_seed) {
                    Some(retry) => sleeper.sleep(retry.delay).await,
                    None => return Err(DownloadError::Provider(error)),
                }
            }
        }
    }
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
