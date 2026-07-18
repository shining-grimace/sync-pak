//! Reusable provider behavior checks for isolated test buckets and prefixes.

use crate::provider_capabilities::{
    BucketLister, ObjectDeleter, ObjectLister, ObjectMetadataReader, ObjectReader, ObjectWriter,
    ProviderError, ProviderResult,
};

const CONTENTS: &[u8] = b"SyncPak provider conformance check\n";

pub async fn verify_bucket_listing<T>(provider: &T, expected_bucket: &str) -> ProviderResult<()>
where
    T: BucketLister,
{
    provider
        .list_buckets()
        .await?
        .contains(&expected_bucket.to_owned())
        .then_some(())
        .ok_or(ProviderError::Unexpected)
}

/// Verifies list, write, read, metadata, and deletion for one generated key.
///
/// The key must be inside a disposable, isolated test prefix. Once writing succeeds, this
/// function attempts cleanup even when an intermediate assertion or provider call fails.
pub async fn verify_object_lifecycle<T>(
    provider: &T,
    bucket: &str,
    prefix: &str,
    key: &str,
) -> ProviderResult<()>
where
    T: ObjectDeleter + ObjectLister + ObjectMetadataReader + ObjectReader + ObjectWriter,
{
    provider.write(bucket, key, CONTENTS).await?;
    let verification = verify_written_object(provider, bucket, prefix, key).await;
    let cleanup = provider.delete(bucket, key).await;
    match (verification, cleanup) {
        (_, Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Ok(())) => verify_deleted_object(provider, bucket, prefix, key).await,
    }
}

async fn verify_written_object<T>(
    provider: &T,
    bucket: &str,
    prefix: &str,
    key: &str,
) -> ProviderResult<()>
where
    T: ObjectLister + ObjectMetadataReader + ObjectReader,
{
    provider
        .list_objects(bucket, prefix)
        .await?
        .iter()
        .any(|object| object.key == key && object.metadata.byte_size == CONTENTS.len() as u64)
        .then_some(())
        .ok_or(ProviderError::Unexpected)?;
    (provider.read(bucket, key).await? == CONTENTS)
        .then_some(())
        .ok_or(ProviderError::Unexpected)?;
    (provider.metadata(bucket, key).await?.byte_size == CONTENTS.len() as u64)
        .then_some(())
        .ok_or(ProviderError::Unexpected)
}

async fn verify_deleted_object<T>(
    provider: &T,
    bucket: &str,
    prefix: &str,
    key: &str,
) -> ProviderResult<()>
where
    T: ObjectLister,
{
    (!provider
        .list_objects(bucket, prefix)
        .await?
        .iter()
        .any(|object| object.key == key))
    .then_some(())
    .ok_or(ProviderError::Unexpected)
}

#[cfg(test)]
#[path = "provider_conformance_tests.rs"]
mod tests;
