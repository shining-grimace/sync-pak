use std::error::Error;
use std::fmt;
use std::path::Path;
use std::time::UNIX_EPOCH;

use crate::{
    cancellation::CancellationToken,
    provider_capabilities::{ObjectWriteMetadata, ObjectWriter, ProviderError},
    retry::{RetryPolicy, RetrySleeper},
};

pub async fn upload_from_path<T: ObjectWriter>(
    provider: &T,
    bucket: &str,
    key: &str,
    source: &Path,
) -> Result<(), UploadError> {
    let (contents, write_metadata) = read_upload_source(source)?;
    provider
        .write_with_metadata(bucket, key, &contents, &write_metadata)
        .await
        .map_err(UploadError::Provider)
}

pub async fn upload_from_path_with_retry<T: ObjectWriter, S: RetrySleeper>(
    provider: &T,
    bucket: &str,
    key: &str,
    source: &Path,
    policy: &RetryPolicy,
    sleeper: &S,
    jitter_seed: u64,
) -> Result<(), UploadError> {
    upload_from_path_with_retry_and_cancellation(
        provider,
        bucket,
        key,
        source,
        policy,
        sleeper,
        jitter_seed,
        &CancellationToken::default(),
    )
    .await
}

/// Uploads a file with retry while respecting cancellation between requests.
pub async fn upload_from_path_with_retry_and_cancellation<T: ObjectWriter, S: RetrySleeper>(
    provider: &T,
    bucket: &str,
    key: &str,
    source: &Path,
    policy: &RetryPolicy,
    sleeper: &S,
    jitter_seed: u64,
    cancellation: &CancellationToken,
) -> Result<(), UploadError> {
    let (contents, write_metadata) = read_upload_source(source)?;
    let mut completed_attempts = 0;
    loop {
        cancellation.check().map_err(|_| UploadError::Cancelled)?;
        completed_attempts += 1;
        match provider
            .write_with_metadata(bucket, key, &contents, &write_metadata)
            .await
        {
            Ok(()) => return Ok(()),
            Err(error) => {
                match policy.delay_after_failure(completed_attempts, error, None, jitter_seed) {
                    Some(retry) => {
                        sleeper.sleep(retry.delay).await;
                        cancellation.check().map_err(|_| UploadError::Cancelled)?;
                    }
                    None => return Err(UploadError::Provider(error)),
                }
            }
        }
    }
}

fn read_upload_source(source: &Path) -> Result<(Vec<u8>, ObjectWriteMetadata), UploadError> {
    let metadata = std::fs::metadata(source).map_err(UploadError::Local)?;
    let contents = std::fs::read(source).map_err(UploadError::Local)?;
    Ok((
        contents,
        ObjectWriteMetadata {
            source_modified_unix_seconds: metadata
                .modified()
                .ok()
                .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                .and_then(|duration| duration.as_secs().try_into().ok()),
        },
    ))
}

#[derive(Debug)]
pub enum UploadError {
    Cancelled,
    Provider(ProviderError),
    Local(std::io::Error),
}

impl fmt::Display for UploadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("upload was cancelled"),
            Self::Provider(error) => error.fmt(formatter),
            Self::Local(error) => write!(formatter, "could not read the upload source: {error}"),
        }
    }
}

impl Error for UploadError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Cancelled => None,
            Self::Provider(error) => Some(error),
            Self::Local(error) => Some(error),
        }
    }
}

#[cfg(test)]
#[path = "upload_tests.rs"]
mod tests;
