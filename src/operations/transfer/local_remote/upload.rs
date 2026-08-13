use std::{io::Read, path::Path, time::UNIX_EPOCH};

use crate::{
    inventory::{InventoryEntryKind, RelativePath},
    operations::cancellation::CancellationToken,
    operations::retry::RetrySleeper,
    operations::transfer::local_remote::{LocalRemoteTransfer, LocalRemoteTransferError},
    operations::transfer::multipart_file::{
        upload_file_with_cancellation, upload_reader_with_cancellation,
    },
    operations::transfer::upload::{
        upload_contents_with_retry_and_cancellation, upload_from_path_with_retry_and_cancellation,
    },
    operations::transfer::upload_strategy::{UploadStrategy, select_upload_strategy},
    providers::capabilities::{
        MultipartUploadRequest, MultipartUploader, ObjectMetadataReader, ObjectWriteMetadata,
        ObjectWriter,
    },
};

impl<P: ObjectWriter, S: RetrySleeper> LocalRemoteTransfer<'_, P, S> {
    #[cfg(test)]
    pub async fn upload(
        &self,
        relative: &RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> Result<(), LocalRemoteTransferError> {
        let metadata = self.local_root.metadata(relative).map_err(local_error)?;
        let mut source = self.local_root.open_read(relative).map_err(local_error)?;
        let mut contents = Vec::new();
        source.read_to_end(&mut contents).map_err(local_error)?;
        upload_contents_with_retry_and_cancellation(
            self.provider,
            self.bucket,
            &self.remote_prefix.resolve(relative),
            &contents,
            &ObjectWriteMetadata {
                source_modified_unix_seconds: metadata.modified_unix_seconds,
            },
            self.retry_policy,
            self.sleeper,
            jitter_seed,
            cancellation,
        )
        .await
        .map_err(LocalRemoteTransferError::Upload)
    }
}

impl<P: ObjectWriter + MultipartUploader + ObjectMetadataReader, S: RetrySleeper>
    LocalRemoteTransfer<'_, P, S>
{
    pub async fn upload_auto(
        &self,
        relative: &RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> Result<(), LocalRemoteTransferError> {
        let metadata = self.local_root.metadata(relative).map_err(local_error)?;
        let key = self.remote_prefix.resolve(relative);
        let write_metadata = ObjectWriteMetadata {
            source_modified_unix_seconds: metadata.modified_unix_seconds,
        };
        if metadata.kind == InventoryEntryKind::Directory {
            return upload_contents_with_retry_and_cancellation(
                self.provider,
                self.bucket,
                &format!("{key}/"),
                &[],
                &write_metadata,
                self.retry_policy,
                self.sleeper,
                jitter_seed,
                cancellation,
            )
            .await
            .map_err(LocalRemoteTransferError::Upload);
        }
        let mut source = self.local_root.open_read(relative).map_err(local_error)?;
        let result = match select_upload_strategy(metadata.byte_size) {
            UploadStrategy::SinglePart => {
                let mut contents = Vec::new();
                source.read_to_end(&mut contents).map_err(local_error)?;
                upload_contents_with_retry_and_cancellation(
                    self.provider,
                    self.bucket,
                    &key,
                    &contents,
                    &write_metadata,
                    self.retry_policy,
                    self.sleeper,
                    jitter_seed,
                    cancellation,
                )
                .await
                .map_err(LocalRemoteTransferError::Upload)
            }
            UploadStrategy::Multipart { part_size } => upload_reader_with_cancellation(
                self.provider,
                &MultipartUploadRequest {
                    bucket: self.bucket.into(),
                    key,
                    content_type: None,
                    source_modified_unix_seconds: metadata.modified_unix_seconds,
                },
                &mut source,
                part_size,
                cancellation,
            )
            .await
            .map_err(LocalRemoteTransferError::Multipart),
        };
        result?;
        self.record_upload(relative, &metadata).await;
        Ok(())
    }

    pub(crate) async fn upload_path_auto(
        &self,
        source: &Path,
        relative: &RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> Result<(), LocalRemoteTransferError> {
        let metadata = std::fs::metadata(source).map_err(local_error)?;
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

fn local_error(error: std::io::Error) -> LocalRemoteTransferError {
    LocalRemoteTransferError::Local(error)
}
