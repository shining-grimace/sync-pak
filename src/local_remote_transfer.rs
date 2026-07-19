use std::{error::Error, fmt};

use crate::{
    cancellation::CancellationToken,
    download::{DownloadError, download_to_path_with_retry_and_cancellation},
    inventory::RelativePath,
    provider_capabilities::{ObjectReader, ObjectWriter},
    retry::{RetryPolicy, RetrySleeper},
    transfer_paths::{LocalTransferRoot, RemoteTransferPrefix},
    upload::{UploadError, upload_from_path_with_retry_and_cancellation},
};

/// Transfers individual validated inventory paths between one local root and provider prefix.
pub struct LocalRemoteTransfer<'a, P, S> {
    provider: &'a P,
    bucket: &'a str,
    local_root: LocalTransferRoot,
    remote_prefix: RemoteTransferPrefix,
    retry_policy: &'a RetryPolicy,
    sleeper: &'a S,
}

impl<'a, P, S> LocalRemoteTransfer<'a, P, S> {
    pub fn new(
        provider: &'a P,
        bucket: &'a str,
        local_root: LocalTransferRoot,
        remote_prefix: RemoteTransferPrefix,
        retry_policy: &'a RetryPolicy,
        sleeper: &'a S,
    ) -> Self {
        Self {
            provider,
            bucket,
            local_root,
            remote_prefix,
            retry_policy,
            sleeper,
        }
    }
}

impl<P: ObjectWriter, S: RetrySleeper> LocalRemoteTransfer<'_, P, S> {
    pub async fn upload(
        &self,
        relative: &RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> Result<(), LocalRemoteTransferError> {
        upload_from_path_with_retry_and_cancellation(
            self.provider,
            self.bucket,
            &self.remote_prefix.resolve(relative),
            &self.local_root.resolve(relative),
            self.retry_policy,
            self.sleeper,
            jitter_seed,
            cancellation,
        )
        .await
        .map_err(LocalRemoteTransferError::Upload)
    }
}

impl<P: ObjectReader, S: RetrySleeper> LocalRemoteTransfer<'_, P, S> {
    pub async fn download(
        &self,
        relative: &RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> Result<(), LocalRemoteTransferError> {
        download_to_path_with_retry_and_cancellation(
            self.provider,
            self.bucket,
            &self.remote_prefix.resolve(relative),
            &self.local_root.resolve(relative),
            self.retry_policy,
            self.sleeper,
            jitter_seed,
            cancellation,
        )
        .await
        .map_err(LocalRemoteTransferError::Download)
    }
}

#[derive(Debug)]
pub enum LocalRemoteTransferError {
    Upload(UploadError),
    Download(DownloadError),
}

impl fmt::Display for LocalRemoteTransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Upload(error) => error.fmt(formatter),
            Self::Download(error) => error.fmt(formatter),
        }
    }
}

impl Error for LocalRemoteTransferError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Upload(error) => Some(error),
            Self::Download(error) => Some(error),
        }
    }
}

#[cfg(test)]
#[path = "local_remote_transfer_tests.rs"]
mod tests;
