//! Multipart provider conformance checks for an isolated test prefix.

use crate::{
    provider_capabilities::{
        MultipartUpload, MultipartUploadRequest, MultipartUploader, ObjectDeleter, ObjectReader,
        ProviderError,
    },
    provider_conformance::{ConformanceError, ConformancePhase},
};

const FIRST_PART_SIZE: usize = 5 * 1024 * 1024;
const SECOND_PART: &[u8] = b"SyncPak multipart conformance check\n";

/// Verifies multipart completion, readback, deletion, and abort behavior.
pub async fn verify_multipart_lifecycle<T>(
    provider: &T,
    bucket: &str,
    key: &str,
) -> Result<(), ConformanceError>
where
    T: MultipartUploader + ObjectDeleter + ObjectReader,
{
    let first_part = vec![b'S'; FIRST_PART_SIZE];
    let expected = [first_part.as_slice(), SECOND_PART].concat();
    let request = request(bucket, key);
    let upload = provider
        .begin_multipart_upload(&request)
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::MultipartBegin, error))?;
    match complete_upload(provider, bucket, key, &upload, &first_part).await {
        Ok(()) => verify_completed_upload(provider, bucket, key, &expected).await?,
        Err(error) => return abort_after_failure(provider, bucket, key, &upload, error).await,
    }
    verify_abort(provider, bucket, &format!("{key}.abort")).await
}

async fn complete_upload<T>(
    provider: &T,
    bucket: &str,
    key: &str,
    upload: &MultipartUpload,
    first_part: &[u8],
) -> Result<(), ConformanceError>
where
    T: MultipartUploader,
{
    let first = provider
        .upload_part(bucket, key, upload, 1, first_part)
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::MultipartPartUpload, error))?;
    let second = provider
        .upload_part(bucket, key, upload, 2, SECOND_PART)
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::MultipartPartUpload, error))?;
    provider
        .complete_multipart_upload(bucket, key, upload, &[first, second])
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::MultipartCompletion, error))
}

async fn verify_completed_upload<T>(
    provider: &T,
    bucket: &str,
    key: &str,
    expected: &[u8],
) -> Result<(), ConformanceError>
where
    T: ObjectDeleter + ObjectReader,
{
    let verification = (provider
        .read(bucket, key)
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::MultipartContent, error))?
        == expected)
        .then_some(())
        .ok_or(ConformanceError::new(
            ConformancePhase::MultipartContent,
            ProviderError::Unexpected,
        ));
    let cleanup = provider
        .delete(bucket, key)
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::MultipartCleanup, error));
    match (verification, cleanup) {
        (_, Err(error)) => Err(error),
        (Err(error), Ok(())) => Err(error),
        (Ok(()), Ok(())) => Ok(()),
    }
}

async fn abort_after_failure<T>(
    provider: &T,
    bucket: &str,
    key: &str,
    upload: &MultipartUpload,
    failure: ConformanceError,
) -> Result<(), ConformanceError>
where
    T: MultipartUploader,
{
    provider
        .abort_multipart_upload(bucket, key, upload)
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::MultipartAbort, error))?;
    Err(failure)
}

async fn verify_abort<T>(provider: &T, bucket: &str, key: &str) -> Result<(), ConformanceError>
where
    T: MultipartUploader,
{
    let upload = provider
        .begin_multipart_upload(&request(bucket, key))
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::MultipartBegin, error))?;
    provider
        .abort_multipart_upload(bucket, key, &upload)
        .await
        .map_err(|error| ConformanceError::new(ConformancePhase::MultipartAbort, error))
}

fn request(bucket: &str, key: &str) -> MultipartUploadRequest {
    MultipartUploadRequest {
        bucket: bucket.to_owned(),
        key: key.to_owned(),
        content_type: Some("application/octet-stream".to_owned()),
    }
}
