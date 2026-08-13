use crate::{
    inventory::{InventoryEntryKind, RelativePath},
    providers::capabilities::{ObjectMetadata, ObjectMetadataReader},
    sync_cache::{Baseline, LocalFingerprint, RemoteIdentity, RemoteObservation},
};

use super::LocalRemoteTransfer;
use super::root_metadata::LocalEntryMetadata;

impl<P, S> LocalRemoteTransfer<'_, P, S> {
    pub(crate) fn invalidate_cache(&self, relative: &RelativePath) {
        let Some(cache) = &self.cache else { return };
        cache.cache.invalidate(
            &cache.namespace,
            self.bucket,
            &self.remote_prefix.resolve(relative),
            relative.as_str(),
        );
    }

    pub(crate) fn record_download(
        &self,
        relative: &RelativePath,
        metadata: Option<&ObjectMetadata>,
    ) {
        let (Some(cache), Some(metadata)) = (&self.cache, metadata) else {
            return;
        };
        let Ok(local) = self.local_root.metadata(relative) else {
            return;
        };
        if local.kind != InventoryEntryKind::File {
            return;
        }
        let remote = RemoteIdentity::from_metadata(metadata);
        let observation = RemoteObservation {
            namespace: cache.namespace.clone(),
            bucket: self.bucket.to_owned(),
            key: self.remote_prefix.resolve(relative),
            identity: remote.clone(),
            source_modified_unix_seconds: metadata.source_modified_unix_seconds,
        };
        let baseline = Baseline {
            namespace: cache.namespace.clone(),
            path: relative.as_str().to_owned(),
            local: LocalFingerprint {
                kind: local.kind,
                byte_size: local.byte_size,
                modified_unix_seconds: local.modified_unix_seconds,
            },
            remote,
            effective_source_unix_seconds: effective_time(metadata),
        };
        cache.cache.record_transfer(&observation, &baseline);
    }
}

impl<P: ObjectMetadataReader, S> LocalRemoteTransfer<'_, P, S> {
    pub(crate) async fn record_upload(
        &self,
        relative: &RelativePath,
        uploaded_local: &LocalEntryMetadata,
    ) {
        let Some(cache) = &self.cache else { return };
        let Ok(local) = self.local_root.metadata(relative) else {
            return;
        };
        if local.kind != InventoryEntryKind::File || local != *uploaded_local {
            return;
        }
        let key = self.remote_prefix.resolve(relative);
        let Ok(metadata) = self.provider.metadata(self.bucket, &key).await else {
            return;
        };
        if metadata.byte_size != uploaded_local.byte_size
            || metadata.source_modified_unix_seconds != uploaded_local.modified_unix_seconds
        {
            return;
        }
        let remote = RemoteIdentity::from_metadata(&metadata);
        let observation = RemoteObservation {
            namespace: cache.namespace.clone(),
            bucket: self.bucket.to_owned(),
            key,
            identity: remote.clone(),
            source_modified_unix_seconds: metadata.source_modified_unix_seconds,
        };
        let baseline = Baseline {
            namespace: cache.namespace.clone(),
            path: relative.as_str().to_owned(),
            local: LocalFingerprint {
                kind: local.kind,
                byte_size: local.byte_size,
                modified_unix_seconds: local.modified_unix_seconds,
            },
            remote,
            effective_source_unix_seconds: effective_time(&metadata),
        };
        cache.cache.record_transfer(&observation, &baseline);
    }
}

pub(crate) fn effective_time(metadata: &ObjectMetadata) -> Option<i64> {
    metadata
        .source_modified_unix_seconds
        .or(metadata.modified_unix_seconds)
}
