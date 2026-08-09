use std::{error::Error, fmt};

use crate::{
    download::DownloadError,
    inventory::InventoryError,
    multipart_file_upload::MultipartFileUploadError,
    retry::RetryPolicy,
    transfer_paths::{LocalTransferRoot, RemoteTransferPrefix},
    upload::UploadError,
};

/// Transfers individual validated inventory paths between one local root and provider prefix.
pub struct LocalRemoteTransfer<'a, P, S> {
    pub(crate) provider: &'a P,
    pub(crate) bucket: &'a str,
    pub(crate) local_root: LocalTransferRoot,
    pub(crate) remote_prefix: RemoteTransferPrefix,
    pub(crate) retry_policy: &'a RetryPolicy,
    pub(crate) sleeper: &'a S,
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

#[derive(Debug)]
pub enum LocalRemoteTransferError {
    UnsupportedDirection,
    Local(std::io::Error),
    Upload(UploadError),
    Multipart(MultipartFileUploadError),
    Download(DownloadError),
    Delete(crate::transfer_delete::TransferDeleteError),
    ArchiveLocation(InventoryError),
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
            Self::ArchiveLocation(error) => {
                write!(formatter, "invalid archive record location: {error}")
            }
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
            Self::ArchiveLocation(error) => Some(error),
        }
    }
}

#[cfg(test)]
#[path = "local_remote_transfer_tests.rs"]
mod tests;
