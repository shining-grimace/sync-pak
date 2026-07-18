use std::error::Error;
use std::fmt;

use crate::{
    cancellation::CancellationToken,
    provider_capabilities::{
        MultipartUpload, MultipartUploadRequest, MultipartUploader, ProviderError, UploadedPart,
    },
};

pub async fn upload_parts<T: MultipartUploader>(
    provider: &T,
    request: &MultipartUploadRequest,
    parts: &[&[u8]],
) -> Result<(), MultipartUploadError> {
    upload_parts_with_cancellation(provider, request, parts, &CancellationToken::default()).await
}

/// Uploads parts until cancellation is requested at a part boundary.
pub async fn upload_parts_with_cancellation<T: MultipartUploader>(
    provider: &T,
    request: &MultipartUploadRequest,
    parts: &[&[u8]],
    cancellation: &CancellationToken,
) -> Result<(), MultipartUploadError> {
    if parts.is_empty() || parts.len() > 10_000 {
        return Err(MultipartUploadError::provider(
            ProviderError::InvalidRequest,
        ));
    }
    cancellation
        .check()
        .map_err(|_| MultipartUploadError::Cancelled { abort_error: None })?;
    let upload = provider
        .begin_multipart_upload(request)
        .await
        .map_err(MultipartUploadError::provider)?;
    let result = upload_all_parts(provider, request, &upload, parts, cancellation).await;
    match result {
        Ok(uploaded_parts) => match provider
            .complete_multipart_upload(&request.bucket, &request.key, &upload, &uploaded_parts)
            .await
        {
            Ok(()) => Ok(()),
            Err(error) => Err(abort_after_failure(
                provider,
                request,
                &upload,
                MultipartUploadError::provider(error),
            )
            .await),
        },
        Err(error) => Err(abort_after_failure(provider, request, &upload, error).await),
    }
}

async fn upload_all_parts<T: MultipartUploader>(
    provider: &T,
    request: &MultipartUploadRequest,
    upload: &MultipartUpload,
    parts: &[&[u8]],
    cancellation: &CancellationToken,
) -> Result<Vec<UploadedPart>, MultipartUploadError> {
    let mut uploaded = Vec::with_capacity(parts.len());
    for (index, contents) in parts.iter().enumerate() {
        cancellation
            .check()
            .map_err(|_| MultipartUploadError::Cancelled { abort_error: None })?;
        let part_number = u32::try_from(index + 1)
            .map_err(|_| MultipartUploadError::provider(ProviderError::InvalidRequest))?;
        uploaded.push(
            provider
                .upload_part(&request.bucket, &request.key, upload, part_number, contents)
                .await
                .map_err(MultipartUploadError::provider)?,
        );
    }
    Ok(uploaded)
}

async fn abort_after_failure<T: MultipartUploader>(
    provider: &T,
    request: &MultipartUploadRequest,
    upload: &MultipartUpload,
    error: MultipartUploadError,
) -> MultipartUploadError {
    let abort_error = provider
        .abort_multipart_upload(&request.bucket, &request.key, upload)
        .await
        .err();
    error.with_abort_error(abort_error)
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MultipartUploadError {
    Cancelled {
        abort_error: Option<ProviderError>,
    },
    Provider {
        error: ProviderError,
        abort_error: Option<ProviderError>,
    },
}

impl fmt::Display for MultipartUploadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled { abort_error } => match abort_error {
                Some(abort_error) => write!(
                    formatter,
                    "multipart upload was cancelled; cleanup also failed: {abort_error}"
                ),
                None => formatter.write_str("multipart upload was cancelled"),
            },
            Self::Provider { error, abort_error } => match abort_error {
                Some(abort_error) => write!(
                    formatter,
                    "multipart upload failed: {error}; cleanup also failed: {abort_error}"
                ),
                None => write!(formatter, "multipart upload failed: {error}"),
            },
        }
    }
}

impl Error for MultipartUploadError {}

impl MultipartUploadError {
    fn provider(error: ProviderError) -> Self {
        Self::Provider {
            error,
            abort_error: None,
        }
    }

    fn with_abort_error(self, abort_error: Option<ProviderError>) -> Self {
        match self {
            Self::Cancelled { .. } => Self::Cancelled { abort_error },
            Self::Provider { error, .. } => Self::Provider { error, abort_error },
        }
    }
}

#[cfg(test)]
#[path = "multipart_upload_tests.rs"]
mod tests;
