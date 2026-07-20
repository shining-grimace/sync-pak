use std::future::Future;

use crate::{
    add_only_execution::AddOnlyTransfer,
    archive_download::ArchiveDownloader,
    archive_prune::ArchiveRemover,
    archive_retention::ArchiveRecord,
    archive_upload::ArchiveUploader,
    cancellation::CancellationToken,
    local_remote_transfer::{LocalRemoteTransfer, LocalRemoteTransferError},
    mirror_execution::MirrorTransfer,
    planning::Direction,
    provider_capabilities::{MultipartUploader, ObjectDeleter, ObjectReader, ObjectWriter},
    retry::RetrySleeper,
    transfer_delete::{delete_local, delete_remote_with_retry_and_cancellation},
};

impl<P: ObjectDeleter, S: RetrySleeper> ArchiveRemover for LocalRemoteTransfer<'_, P, S> {
    type Error = LocalRemoteTransferError;

    fn remove(&self, archive: &ArchiveRecord) -> impl Future<Output = Result<(), Self::Error>> {
        async move {
            let path = crate::inventory::RelativePath::new(&archive.location)
                .map_err(LocalRemoteTransferError::ArchiveLocation)?;
            delete_remote_with_retry_and_cancellation(
                self.provider,
                self.bucket,
                &self.remote_prefix,
                &path,
                self.retry_policy,
                self.sleeper,
                0,
                &CancellationToken::default(),
            )
            .await
            .map_err(LocalRemoteTransferError::Delete)
        }
    }
}

impl<P: ObjectReader, S: RetrySleeper> ArchiveDownloader for LocalRemoteTransfer<'_, P, S> {
    type Error = LocalRemoteTransferError;

    fn download(
        &self,
        source: &crate::inventory::RelativePath,
        destination: &std::path::Path,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async move {
            self.download_path(source, destination, cancellation, jitter_seed)
                .await
        }
    }
}

impl<P: ObjectWriter + MultipartUploader, S: RetrySleeper> ArchiveUploader
    for LocalRemoteTransfer<'_, P, S>
{
    type Error = LocalRemoteTransferError;

    fn upload(
        &self,
        source: &std::path::Path,
        destination: &crate::inventory::RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async move {
            self.upload_path_auto(source, destination, cancellation, jitter_seed)
                .await
        }
    }
}

impl<P: ObjectReader + ObjectWriter + MultipartUploader, S: RetrySleeper> AddOnlyTransfer
    for LocalRemoteTransfer<'_, P, S>
{
    type Error = LocalRemoteTransferError;

    fn upload(
        &self,
        path: &crate::inventory::RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async move { self.upload_auto(path, cancellation, jitter_seed).await }
    }

    fn download(
        &self,
        path: &crate::inventory::RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async move { LocalRemoteTransfer::download(self, path, cancellation, jitter_seed).await }
    }
}

impl<P: ObjectDeleter + ObjectReader + ObjectWriter + MultipartUploader, S: RetrySleeper>
    MirrorTransfer for LocalRemoteTransfer<'_, P, S>
{
    type Error = LocalRemoteTransferError;

    fn copy(
        &self,
        direction: Direction,
        path: &crate::inventory::RelativePath,
        _: bool,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async move {
            match direction {
                Direction::Upload => self.upload_auto(path, cancellation, jitter_seed).await,
                Direction::Download => {
                    LocalRemoteTransfer::download(self, path, cancellation, jitter_seed).await
                }
                Direction::BothWays => Err(LocalRemoteTransferError::UnsupportedDirection),
            }
        }
    }

    fn delete(
        &self,
        direction: Direction,
        path: &crate::inventory::RelativePath,
        cancellation: &CancellationToken,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async move {
            match direction {
                Direction::Upload => delete_remote_with_retry_and_cancellation(
                    self.provider,
                    self.bucket,
                    &self.remote_prefix,
                    path,
                    self.retry_policy,
                    self.sleeper,
                    0,
                    cancellation,
                )
                .await
                .map_err(LocalRemoteTransferError::Delete),
                Direction::Download => delete_local(&self.local_root, path, cancellation)
                    .map_err(LocalRemoteTransferError::Delete),
                Direction::BothWays => Err(LocalRemoteTransferError::UnsupportedDirection),
            }
        }
    }
}
