use std::{error::Error, fmt};

use crate::{
    cancellation::CancellationToken,
    inventory::RelativePath,
    provider_capabilities::{ObjectDeleter, ProviderError},
    retry::{RetryPolicy, RetrySleeper},
    transfer_paths::{LocalTransferRoot, RemoteTransferPrefix},
};

/// Removes one local file, symlink, or already-empty directory beneath a transfer root.
pub fn delete_local(
    root: &LocalTransferRoot,
    relative: &RelativePath,
    cancellation: &CancellationToken,
) -> Result<(), TransferDeleteError> {
    cancellation
        .check()
        .map_err(|_| TransferDeleteError::Cancelled)?;
    let path = root.resolve(relative);
    let metadata = match std::fs::symlink_metadata(&path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(TransferDeleteError::Local(error)),
    };
    let result = if metadata.file_type().is_dir() {
        std::fs::remove_dir(path)
    } else {
        std::fs::remove_file(path)
    };
    result.map_err(TransferDeleteError::Local)
}

/// Removes one provider object beneath a transfer prefix.
pub async fn delete_remote<T: ObjectDeleter>(
    provider: &T,
    bucket: &str,
    prefix: &RemoteTransferPrefix,
    relative: &RelativePath,
    cancellation: &CancellationToken,
) -> Result<(), TransferDeleteError> {
    cancellation
        .check()
        .map_err(|_| TransferDeleteError::Cancelled)?;
    provider
        .delete(bucket, &prefix.resolve(relative))
        .await
        .map_err(TransferDeleteError::Provider)
}

/// Removes a provider object with bounded retry while respecting cancellation.
pub async fn delete_remote_with_retry_and_cancellation<T: ObjectDeleter, S: RetrySleeper>(
    provider: &T,
    bucket: &str,
    prefix: &RemoteTransferPrefix,
    relative: &RelativePath,
    policy: &RetryPolicy,
    sleeper: &S,
    jitter_seed: u64,
    cancellation: &CancellationToken,
) -> Result<(), TransferDeleteError> {
    let mut completed_attempts = 0;
    loop {
        cancellation
            .check()
            .map_err(|_| TransferDeleteError::Cancelled)?;
        completed_attempts += 1;
        match provider.delete(bucket, &prefix.resolve(relative)).await {
            Ok(()) => return Ok(()),
            Err(error) => {
                match policy.delay_after_failure(completed_attempts, error, None, jitter_seed) {
                    Some(retry) => {
                        sleeper.sleep(retry.delay).await;
                        cancellation
                            .check()
                            .map_err(|_| TransferDeleteError::Cancelled)?;
                    }
                    None => return Err(TransferDeleteError::Provider(error)),
                }
            }
        }
    }
}

#[derive(Debug)]
pub enum TransferDeleteError {
    Cancelled,
    Local(std::io::Error),
    Provider(ProviderError),
}

impl fmt::Display for TransferDeleteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("deletion was cancelled"),
            Self::Local(error) => {
                write!(formatter, "could not remove the local destination: {error}")
            }
            Self::Provider(error) => error.fmt(formatter),
        }
    }
}

impl Error for TransferDeleteError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Cancelled => None,
            Self::Local(error) => Some(error),
            Self::Provider(error) => Some(error),
        }
    }
}

#[cfg(test)]
#[path = "transfer_delete_tests.rs"]
mod tests;
