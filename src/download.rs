use std::error::Error;
use std::fmt;
use std::path::Path;

use crate::{
    atomic_write::atomic_write,
    cancellation::CancellationToken,
    provider_capabilities::{ObjectReader, ProviderError},
    retry::{NoopRetryObserver, RetryObserver, RetryPolicy, RetrySleeper},
};

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

/// Downloads with bounded retry and reports each scheduled retry delay.
pub async fn download_to_path_with_retry_and_cancellation_and_observer<
    T: ObjectReader,
    S: RetrySleeper,
    O: RetryObserver,
>(
    provider: &T,
    bucket: &str,
    key: &str,
    destination: &Path,
    policy: &RetryPolicy,
    sleeper: &S,
    jitter_seed: u64,
    cancellation: &CancellationToken,
    observer: &O,
) -> Result<(), DownloadError> {
    let mut completed_attempts = 0;
    loop {
        cancellation.check().map_err(|_| DownloadError::Cancelled)?;
        completed_attempts += 1;
        match provider.read(bucket, key).await {
            Ok(contents) => {
                cancellation.check().map_err(|_| DownloadError::Cancelled)?;
                return atomic_write(destination, &contents).map_err(DownloadError::Local);
            }
            Err(error) => {
                match policy.delay_after_failure(completed_attempts, error, None, jitter_seed) {
                    Some(retry) => {
                        observer.on_retry(retry);
                        sleeper.sleep(retry.delay).await;
                        cancellation.check().map_err(|_| DownloadError::Cancelled)?;
                    }
                    None => return Err(DownloadError::Provider(error)),
                }
            }
        }
    }
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
    download_to_path_with_retry_and_cancellation(
        provider,
        bucket,
        key,
        destination,
        policy,
        sleeper,
        jitter_seed,
        &CancellationToken::default(),
    )
    .await
}

/// Downloads with retry while respecting cancellation before requests and replacement.
pub async fn download_to_path_with_retry_and_cancellation<T: ObjectReader, S: RetrySleeper>(
    provider: &T,
    bucket: &str,
    key: &str,
    destination: &Path,
    policy: &RetryPolicy,
    sleeper: &S,
    jitter_seed: u64,
    cancellation: &CancellationToken,
) -> Result<(), DownloadError> {
    download_to_path_with_retry_and_cancellation_and_observer(
        provider,
        bucket,
        key,
        destination,
        policy,
        sleeper,
        jitter_seed,
        cancellation,
        &NoopRetryObserver,
    )
    .await
}

#[derive(Debug)]
pub enum DownloadError {
    Cancelled,
    Provider(ProviderError),
    Local(std::io::Error),
}

impl fmt::Display for DownloadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("download was cancelled"),
            Self::Provider(error) => error.fmt(formatter),
            Self::Local(error) => write!(formatter, "could not save the downloaded file: {error}"),
        }
    }
}

impl Error for DownloadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Cancelled => None,
            Self::Provider(error) => Some(error),
            Self::Local(error) => Some(error),
        }
    }
}

#[cfg(test)]
#[path = "download_tests.rs"]
mod tests;
