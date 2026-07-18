use std::error::Error;
use std::fmt;

use crate::provider_capabilities::{
    MultipartUpload, MultipartUploadRequest, MultipartUploader, ProviderError, UploadedPart,
};

pub async fn upload_parts<T: MultipartUploader>(
    provider: &T,
    request: &MultipartUploadRequest,
    parts: &[&[u8]],
) -> Result<(), MultipartUploadError> {
    if parts.is_empty() || parts.len() > 10_000 {
        return Err(MultipartUploadError::Provider {
            error: ProviderError::InvalidRequest,
            abort_error: None,
        });
    }
    let upload = provider
        .begin_multipart_upload(request)
        .await
        .map_err(|error| MultipartUploadError::Provider {
            error,
            abort_error: None,
        })?;
    let result = upload_all_parts(provider, request, &upload, parts).await;
    match result {
        Ok(uploaded_parts) => match provider
            .complete_multipart_upload(&request.bucket, &request.key, &upload, &uploaded_parts)
            .await
        {
            Ok(()) => Ok(()),
            Err(error) => Err(abort_after_failure(provider, request, &upload, error).await),
        },
        Err(error) => Err(abort_after_failure(provider, request, &upload, error).await),
    }
}

async fn upload_all_parts<T: MultipartUploader>(
    provider: &T,
    request: &MultipartUploadRequest,
    upload: &MultipartUpload,
    parts: &[&[u8]],
) -> Result<Vec<UploadedPart>, ProviderError> {
    let mut uploaded = Vec::with_capacity(parts.len());
    for (index, contents) in parts.iter().enumerate() {
        let part_number = u32::try_from(index + 1).map_err(|_| ProviderError::InvalidRequest)?;
        uploaded.push(
            provider
                .upload_part(&request.bucket, &request.key, upload, part_number, contents)
                .await?,
        );
    }
    Ok(uploaded)
}

async fn abort_after_failure<T: MultipartUploader>(
    provider: &T,
    request: &MultipartUploadRequest,
    upload: &MultipartUpload,
    error: ProviderError,
) -> MultipartUploadError {
    let abort_error = provider
        .abort_multipart_upload(&request.bucket, &request.key, upload)
        .await
        .err();
    MultipartUploadError::Provider { error, abort_error }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MultipartUploadError {
    Provider {
        error: ProviderError,
        abort_error: Option<ProviderError>,
    },
}

impl fmt::Display for MultipartUploadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
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

#[cfg(test)]
#[path = "multipart_upload_tests.rs"]
mod tests;
