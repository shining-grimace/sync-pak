use std::{error::Error, fmt, path::Path, time::UNIX_EPOCH};

use crate::{
    cancellation::CancellationToken,
    download::{DownloadError, download_to_path_with_retry_and_cancellation},
    inventory::RelativePath,
    multipart_file_upload::{MultipartFileUploadError, upload_file_with_cancellation},
    provider_capabilities::{
        MultipartUploadRequest, MultipartUploader, ObjectReader, ObjectWriter,
    },
    retry::{RetryPolicy, RetrySleeper},
    transfer_paths::{LocalTransferRoot, RemoteTransferPrefix},
    upload::{UploadError, upload_from_path_with_retry_and_cancellation},
    upload_strategy::{UploadStrategy, select_upload_strategy},
};

/// Transfers individual validated inventory paths between one local root and provider prefix.
pub struct LocalRemoteTransfer<'a, P, S> {
    pub(crate) provider: &'a P,
    pub(crate) bucket: &'a str,
    pub(crate) local_root: LocalTransferRoot,
    pub(crate) remote_prefix: RemoteTransferPrefix,
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

impl<P: ObjectWriter + MultipartUploader, S: RetrySleeper> LocalRemoteTransfer<'_, P, S> {
    /// Uploads a file with the bounded-memory multipart strategy when it is large enough.
    pub async fn upload_auto(
        &self,
        relative: &RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> Result<(), LocalRemoteTransferError> {
        let source = self.local_root.resolve(relative);
        self.upload_path_auto(&source, relative, cancellation, jitter_seed)
            .await
    }

    /// Uploads a local file to a validated key using the normal size strategy.
    pub(crate) async fn upload_path_auto(
        &self,
        source: &Path,
        relative: &RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> Result<(), LocalRemoteTransferError> {
        let metadata = std::fs::metadata(&source).map_err(LocalRemoteTransferError::Local)?;
        match select_upload_strategy(metadata.len()) {
            UploadStrategy::SinglePart => upload_from_path_with_retry_and_cancellation(
                self.provider,
                self.bucket,
                &self.remote_prefix.resolve(relative),
                source,
                self.retry_policy,
                self.sleeper,
                jitter_seed,
                cancellation,
            )
            .await
            .map_err(LocalRemoteTransferError::Upload),
            UploadStrategy::Multipart { part_size } => upload_file_with_cancellation(
                self.provider,
                &MultipartUploadRequest {
                    bucket: self.bucket.into(),
                    key: self.remote_prefix.resolve(relative),
                    content_type: None,
                    source_modified_unix_seconds: metadata
                        .modified()
                        .ok()
                        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
                        .and_then(|duration| duration.as_secs().try_into().ok()),
                },
                source,
                part_size,
                cancellation,
            )
            .await
            .map_err(LocalRemoteTransferError::Multipart),
        }
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
    UnsupportedDirection,
    Local(std::io::Error),
    Upload(UploadError),
    Multipart(MultipartFileUploadError),
    Download(DownloadError),
    Delete(crate::transfer_delete::TransferDeleteError),
}

impl fmt::Display for LocalRemoteTransferError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedDirection => {
                formatter.write_str("this operation direction is not supported")
            }
            Self::Local(error) => write!(formatter, "could not inspect the upload source: {error}"),
            Self::Upload(error) => error.fmt(formatter),
            Self::Multipart(error) => error.fmt(formatter),
            Self::Download(error) => error.fmt(formatter),
            Self::Delete(error) => error.fmt(formatter),
        }
    }
}

impl Error for LocalRemoteTransferError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::UnsupportedDirection => None,
            Self::Local(error) => Some(error),
            Self::Upload(error) => Some(error),
            Self::Multipart(error) => Some(error),
            Self::Download(error) => Some(error),
            Self::Delete(error) => Some(error),
        }
    }
}

#[cfg(test)]
#[path = "local_remote_transfer_tests.rs"]
mod tests;
