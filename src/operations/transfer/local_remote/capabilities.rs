use crate::{
    operations::archive::download::ArchiveDownloader,
    operations::archive::prune::ArchiveRemover,
    operations::archive::retention::ArchiveRecord,
    operations::archive::upload::ArchiveUploader,
    operations::cancellation::CancellationToken,
    operations::retry::{RetryContext, RetrySleeper},
    operations::transfer::delete::{delete_local, delete_remote_with_retry_and_cancellation},
    operations::transfer::local_remote::{LocalRemoteTransfer, LocalRemoteTransferError},
    operations::transfer::modes::add_only::AddOnlyTransfer,
    operations::transfer::modes::mirror::MirrorTransfer,
    preflight::planning::Direction,
    providers::capabilities::{
        MultipartUploader, ObjectDeleter, ObjectMetadataReader, ObjectPrefixChecker, ObjectReader,
        ObjectWriter,
    },
};

impl<P: ObjectDeleter, S: RetrySleeper> ArchiveRemover for LocalRemoteTransfer<'_, P, S> {
    type Error = LocalRemoteTransferError;

    async fn remove(
        &self,
        archive: &ArchiveRecord,
        cancellation: &CancellationToken,
    ) -> Result<(), Self::Error> {
        let path = crate::inventory::RelativePath::new(&archive.location)
            .map_err(LocalRemoteTransferError::ArchiveLocation)?;
        delete_remote_with_retry_and_cancellation(
            self.provider,
            self.bucket,
            &self.remote_prefix,
            &path,
            &RetryContext {
                policy: self.retry_policy,
                sleeper: self.sleeper,
                jitter_seed: 0,
                cancellation,
            },
        )
        .await
        .map_err(LocalRemoteTransferError::Delete)?;
        self.invalidate_cache(&path);
        Ok(())
    }
}

impl<P: ObjectReader, S: RetrySleeper> ArchiveDownloader for LocalRemoteTransfer<'_, P, S> {
    type Error = LocalRemoteTransferError;

    async fn download(
        &self,
        source: &crate::inventory::RelativePath,
        destination: &std::path::Path,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> Result<(), Self::Error> {
        self.download_path(source, destination, cancellation, jitter_seed)
            .await
    }
}

impl<P: ObjectWriter + MultipartUploader + ObjectMetadataReader, S: RetrySleeper> ArchiveUploader
    for LocalRemoteTransfer<'_, P, S>
{
    type Error = LocalRemoteTransferError;

    async fn upload(
        &self,
        source: &std::path::Path,
        destination: &crate::inventory::RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> Result<(), Self::Error> {
        self.upload_path_auto(source, destination, cancellation, jitter_seed)
            .await
    }
}

impl<
    P: ObjectPrefixChecker + ObjectReader + ObjectWriter + MultipartUploader + ObjectMetadataReader,
    S: RetrySleeper,
> AddOnlyTransfer for LocalRemoteTransfer<'_, P, S>
{
    type Error = LocalRemoteTransferError;

    async fn upload(
        &self,
        path: &crate::inventory::RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> Result<(), Self::Error> {
        self.upload_auto(path, cancellation, jitter_seed).await
    }

    async fn download(
        &self,
        path: &crate::inventory::RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> Result<(), Self::Error> {
        LocalRemoteTransfer::download(self, path, cancellation, jitter_seed).await
    }

    async fn accept_existing(
        &self,
        path: &crate::inventory::RelativePath,
    ) -> Result<(), Self::Error> {
        self.record_accepted_pair(path).await;
        Ok(())
    }
}

impl<
    P: ObjectDeleter
        + ObjectPrefixChecker
        + ObjectReader
        + ObjectWriter
        + MultipartUploader
        + ObjectMetadataReader,
    S: RetrySleeper,
> MirrorTransfer for LocalRemoteTransfer<'_, P, S>
{
    type Error = LocalRemoteTransferError;

    async fn copy(
        &self,
        direction: Direction,
        path: &crate::inventory::RelativePath,
        _: bool,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> Result<(), Self::Error> {
        match direction {
            Direction::Upload => self.upload_auto(path, cancellation, jitter_seed).await,
            Direction::Download => {
                LocalRemoteTransfer::download(self, path, cancellation, jitter_seed).await
            }
            Direction::BothWays => Err(LocalRemoteTransferError::UnsupportedDirection),
        }
    }

    async fn delete(
        &self,
        direction: Direction,
        path: &crate::inventory::RelativePath,
        cancellation: &CancellationToken,
    ) -> Result<(), Self::Error> {
        let result = match direction {
            Direction::Upload => delete_remote_with_retry_and_cancellation(
                self.provider,
                self.bucket,
                &self.remote_prefix,
                path,
                &RetryContext {
                    policy: self.retry_policy,
                    sleeper: self.sleeper,
                    jitter_seed: 0,
                    cancellation,
                },
            )
            .await
            .map_err(LocalRemoteTransferError::Delete),
            Direction::Download => delete_local(&self.local_root, path, cancellation)
                .map_err(LocalRemoteTransferError::Delete),
            Direction::BothWays => Err(LocalRemoteTransferError::UnsupportedDirection),
        };
        result?;
        self.invalidate_cache(path);
        Ok(())
    }
}
