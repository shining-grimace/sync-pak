//! Reusable provider behavior checks for isolated test buckets and prefixes.

use crate::provider_capabilities::{
    BucketLister, ObjectDeleter, ObjectLister, ObjectMetadataReader, ObjectReader, ObjectWriter,
    ProviderError,
};

const CONTENTS: &[u8] = b"SyncPak provider conformance check\n";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ConformancePhase {
    BucketListing,
    ObjectWrite,
    ObjectListing,
    ObjectRead,
    ObjectContent,
    ObjectMetadata,
    ObjectDeletion,
    DeletionVerification,
    MultipartBegin,
    MultipartPartUpload,
    MultipartCompletion,
    MultipartContent,
    MultipartCleanup,
    MultipartAbort,
}

impl std::fmt::Display for ConformancePhase {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::BucketListing => "bucket listing",
            Self::ObjectWrite => "object upload",
            Self::ObjectListing => "object listing",
            Self::ObjectRead => "object download",
            Self::ObjectContent => "download verification",
            Self::ObjectMetadata => "object metadata",
            Self::ObjectDeletion => "object cleanup",
            Self::DeletionVerification => "cleanup verification",
            Self::MultipartBegin => "multipart upload start",
            Self::MultipartPartUpload => "multipart part upload",
            Self::MultipartCompletion => "multipart upload completion",
            Self::MultipartContent => "multipart download verification",
            Self::MultipartCleanup => "multipart object cleanup",
            Self::MultipartAbort => "multipart upload abort",
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConformanceError {
    pub phase: ConformancePhase,
    pub provider_error: ProviderError,
}

impl ConformanceError {
    pub(crate) fn new(phase: ConformancePhase, provider_error: ProviderError) -> Self {
        Self {
            phase,
            provider_error,
        }
    }
}

impl std::fmt::Display for ConformanceError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{} failed: {}", self.phase, self.provider_error)
    }
}

impl std::error::Error for ConformanceError {}

pub async fn verify_bucket_listing<T>(
    provider: &T,
    expected_bucket: &str,
) -> Result<(), ConformanceError>
where
    T: BucketLister,
{
    provider
        .list_buckets()
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::BucketListing, error))?
        .contains(&expected_bucket.to_owned())
        .then_some(())
        .ok_or(ConformanceError::new(
            ConformancePhase::BucketListing,
            ProviderError::Unexpected,
        ))
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
) -> Result<(), ConformanceError>
where
    T: ObjectDeleter + ObjectLister + ObjectMetadataReader + ObjectReader + ObjectWriter,
{
    provider
        .write(bucket, key, CONTENTS)
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::ObjectWrite, error))?;
    let verification = verify_written_object(provider, bucket, prefix, key).await;
    let cleanup = provider
        .delete(bucket, key)
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::ObjectDeletion, error));
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
) -> Result<(), ConformanceError>
where
    T: ObjectLister + ObjectMetadataReader + ObjectReader,
{
    provider
        .list_objects(bucket, prefix)
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::ObjectListing, error))?
        .iter()
        .any(|object| object.key == key && object.metadata.byte_size == CONTENTS.len() as u64)
        .then_some(())
        .ok_or(ConformanceError::new(
            ConformancePhase::ObjectListing,
            ProviderError::Unexpected,
        ))?;
    (provider
        .read(bucket, key)
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::ObjectRead, error))?
        == CONTENTS)
        .then_some(())
        .ok_or(ConformanceError::new(
            ConformancePhase::ObjectContent,
            ProviderError::Unexpected,
        ))?;
    (provider
        .metadata(bucket, key)
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::ObjectMetadata, error))?
        .byte_size
        == CONTENTS.len() as u64)
        .then_some(())
        .ok_or(ConformanceError::new(
            ConformancePhase::ObjectMetadata,
            ProviderError::Unexpected,
        ))
}

async fn verify_deleted_object<T>(
    provider: &T,
    bucket: &str,
    prefix: &str,
    key: &str,
) -> Result<(), ConformanceError>
where
    T: ObjectLister,
{
    (!provider
        .list_objects(bucket, prefix)
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::DeletionVerification, error))?
        .iter()
        .any(|object| object.key == key))
    .then_some(())
    .ok_or(ConformanceError::new(
        ConformancePhase::DeletionVerification,
        ProviderError::Unexpected,
    ))
}

#[cfg(test)]
#[path = "provider_conformance_tests.rs"]
mod tests;

#[cfg(test)]
#[path = "provider_multipart_conformance_tests.rs"]
mod multipart_tests;
