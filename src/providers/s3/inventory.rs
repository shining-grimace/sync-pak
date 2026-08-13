use futures_util::{StreamExt, TryStreamExt, stream};

use crate::{
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    preflight::comparison::modification_times_match,
    providers::capabilities::{ObjectLister, ObjectMetadataReader, ProviderError, RemoteObject},
    providers::s3::transport::S3Transport,
    sync_cache::{
        CacheNamespace, CacheSnapshot, LocalFingerprint, RemoteIdentity, RemoteObservation,
        SyncCache,
    },
};

const METADATA_CONCURRENCY: usize = 8;

impl S3Transport {
    pub async fn comparison_objects(
        &self,
        bucket: &str,
        prefix: &str,
        local: &Inventory,
        cache: Option<((&SyncCache, &CacheNamespace), &CacheSnapshot)>,
    ) -> Result<Vec<RemoteObject>, ProviderError> {
        let objects = self.list_objects(bucket, prefix).await?;
        let enriched: Vec<_> = stream::iter(objects.into_iter().map(|object| async move {
            self.enrich_candidate(bucket, prefix, local, object, cache)
                .await
        }))
        .buffer_unordered(METADATA_CONCURRENCY)
        .try_collect()
        .await?;
        if let Some(((cache, _), _)) = cache {
            let observations = enriched
                .iter()
                .filter_map(|(_, observation)| observation.clone())
                .collect::<Vec<_>>();
            cache.record_observations(&observations);
        }
        Ok(enriched.into_iter().map(|(object, _)| object).collect())
    }

    async fn enrich_candidate(
        &self,
        bucket: &str,
        prefix: &str,
        local: &Inventory,
        mut object: RemoteObject,
        cache: Option<((&SyncCache, &CacheNamespace), &CacheSnapshot)>,
    ) -> Result<(RemoteObject, Option<RemoteObservation>), ProviderError> {
        let Some((path, local)) = candidate(prefix, &object, local) else {
            return Ok((object, None));
        };
        let remote_identity = RemoteIdentity::from_metadata(&object.metadata);
        if modification_times_match(
            local.modified_unix_seconds,
            object.metadata.modified_unix_seconds,
        ) {
            return Ok((object, None));
        }
        if let Some(((_, _), snapshot)) = cache
            && let Some(source_time) = cached_source_time(
                bucket,
                &object.key,
                &path,
                local,
                &remote_identity,
                snapshot,
            )
        {
            object.metadata.source_modified_unix_seconds = source_time;
            return Ok((object, None));
        }
        if let Ok(metadata) = self.metadata(bucket, &object.key).await {
            object.metadata = metadata;
            let observation = cache.map(|((_, namespace), _)| RemoteObservation {
                namespace: namespace.clone(),
                bucket: bucket.to_owned(),
                key: object.key.clone(),
                identity: RemoteIdentity::from_metadata(&object.metadata),
                source_modified_unix_seconds: object.metadata.source_modified_unix_seconds,
            });
            return Ok((object, observation));
        }
        Ok((object, None))
    }
}

fn cached_source_time(
    bucket: &str,
    key: &str,
    path: &RelativePath,
    local: &InventoryEntry,
    remote: &RemoteIdentity,
    snapshot: &CacheSnapshot,
) -> Option<Option<i64>> {
    if !remote.is_cacheable() {
        return None;
    }
    let local = LocalFingerprint {
        kind: local.kind.clone(),
        byte_size: local.byte_size,
        modified_unix_seconds: local.modified_unix_seconds,
    };
    if snapshot
        .baseline(path.as_str())
        .is_some_and(|baseline| baseline.local == local && baseline.remote == *remote)
    {
        return Some(local.modified_unix_seconds);
    }
    snapshot
        .observation(bucket, key)
        .filter(|observation| observation.identity == *remote)
        .map(|observation| observation.source_modified_unix_seconds)
}

fn candidate<'a>(
    prefix: &str,
    object: &RemoteObject,
    inventory: &'a Inventory,
) -> Option<(RelativePath, &'a InventoryEntry)> {
    let relative = object.key.strip_prefix(prefix)?;
    if relative.ends_with('/') {
        return None;
    }
    let path = RelativePath::new(relative).ok()?;
    let local = inventory.get(&path)?;
    (local.kind == InventoryEntryKind::File && local.byte_size == object.metadata.byte_size)
        .then_some((path, local))
}

#[cfg(test)]
mod tests;
