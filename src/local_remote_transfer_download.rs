use std::path::Path;

use crate::{
    cancellation::CancellationToken,
    download::{
        DownloadError, download_contents_with_retry_and_cancellation,
        download_to_path_with_retry_and_cancellation,
    },
    inventory::RelativePath,
    local_remote_transfer::{LocalRemoteTransfer, LocalRemoteTransferError},
    provider_capabilities::{ObjectPrefixChecker, ObjectReader, ProviderError},
    retry::RetrySleeper,
};

impl<P: ObjectPrefixChecker + ObjectReader, S: RetrySleeper> LocalRemoteTransfer<'_, P, S> {
    pub async fn download(
        &self,
        relative: &RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> Result<(), LocalRemoteTransferError> {
        let key = self.remote_prefix.resolve(relative);
        match download_contents_with_retry_and_cancellation(
            self.provider,
            self.bucket,
            &key,
            self.retry_policy,
            self.sleeper,
            jitter_seed,
            cancellation,
        )
        .await
        {
            Ok(contents) => self
                .local_root
                .write(relative, &contents)
                .map_err(LocalRemoteTransferError::Local),
            Err(DownloadError::Provider(ProviderError::NotFound)) => {
                cancellation
                    .check()
                    .map_err(|_| LocalRemoteTransferError::Download(DownloadError::Cancelled))?;
                let has_descendants = self
                    .provider
                    .prefix_exists(self.bucket, &format!("{key}/"))
                    .await
                    .map_err(DownloadError::Provider)
                    .map_err(LocalRemoteTransferError::Download)?;
                if !has_descendants {
                    return Err(LocalRemoteTransferError::Download(DownloadError::Provider(
                        ProviderError::NotFound,
                    )));
                }
                self.local_root
                    .create_directory_all(relative)
                    .map_err(LocalRemoteTransferError::Local)
            }
            Err(error) => Err(LocalRemoteTransferError::Download(error)),
        }
    }
}

impl<P: ObjectReader, S: RetrySleeper> LocalRemoteTransfer<'_, P, S> {
    /// Downloads one validated remote path to an arbitrary local staging destination.
    pub(crate) async fn download_path(
        &self,
        relative: &RelativePath,
        destination: &Path,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> Result<(), LocalRemoteTransferError> {
        download_to_path_with_retry_and_cancellation(
            self.provider,
            self.bucket,
            &self.remote_prefix.resolve(relative),
            destination,
            self.retry_policy,
            self.sleeper,
            jitter_seed,
            cancellation,
        )
        .await
        .map_err(LocalRemoteTransferError::Download)
    }
}
