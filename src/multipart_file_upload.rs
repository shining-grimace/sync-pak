use std::{io::Read, path::Path};

use crate::{
    cancellation::CancellationToken,
    multipart_upload::MultipartUploadError,
    provider_capabilities::{
        MultipartUpload, MultipartUploadRequest, MultipartUploader, ProviderError,
    },
};

pub use crate::multipart_file_upload_error::MultipartFileUploadError;

pub async fn upload_file<T: MultipartUploader>(
    provider: &T,
    request: &MultipartUploadRequest,
    source: &Path,
    part_size: usize,
) -> Result<(), MultipartFileUploadError> {
    upload_file_with_cancellation(
        provider,
        request,
        source,
        part_size,
        &CancellationToken::default(),
    )
    .await
}

/// Uploads a local file in parts until cancellation is requested at a part boundary.
pub async fn upload_file_with_cancellation<T: MultipartUploader>(
    provider: &T,
    request: &MultipartUploadRequest,
    source: &Path,
    part_size: usize,
    cancellation: &CancellationToken,
) -> Result<(), MultipartFileUploadError> {
    let mut file = std::fs::File::open(source).map_err(local_error)?;
    upload_reader_with_cancellation(provider, request, &mut file, part_size, cancellation).await
}

pub async fn upload_reader_with_cancellation<T: MultipartUploader, R: Read>(
    provider: &T,
    request: &MultipartUploadRequest,
    source: &mut R,
    part_size: usize,
    cancellation: &CancellationToken,
) -> Result<(), MultipartFileUploadError> {
    if part_size == 0 {
        return Err(local_error(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "multipart part size must not be zero",
        )));
    }
    cancellation
        .check()
        .map_err(MultipartFileUploadError::from)?;
    let upload = provider
        .begin_multipart_upload(request)
        .await
        .map_err(provider_error)?;
    let mut uploaded = Vec::new();
    loop {
        if let Err(cancelled) = cancellation.check() {
            return Err(abort(provider, request, &upload, cancelled.into()).await);
        }
        let mut buffer = vec![0; part_size];
        let read = match source.read(&mut buffer) {
            Ok(read) => read,
            Err(error) => return Err(abort(provider, request, &upload, local_error(error)).await),
        };
        if read == 0 {
            break;
        }
        buffer.truncate(read);
        let part_number = match u32::try_from(uploaded.len() + 1) {
            Ok(part_number) => part_number,
            Err(_) => {
                return Err(abort(
                    provider,
                    request,
                    &upload,
                    provider_error(ProviderError::InvalidRequest),
                )
                .await);
            }
        };
        match provider
            .upload_part(&request.bucket, &request.key, &upload, part_number, &buffer)
            .await
        {
            Ok(part) => uploaded.push(part),
            Err(error) => {
                return Err(abort(provider, request, &upload, provider_error(error)).await);
            }
        }
    }
    if uploaded.is_empty() {
        return Err(abort(
            provider,
            request,
            &upload,
            provider_error(ProviderError::InvalidRequest),
        )
        .await);
    }
    match provider
        .complete_multipart_upload(&request.bucket, &request.key, &upload, &uploaded)
        .await
    {
        Ok(()) => Ok(()),
        Err(error) => Err(abort(provider, request, &upload, provider_error(error)).await),
    }
}

fn provider_error(error: ProviderError) -> MultipartFileUploadError {
    MultipartFileUploadError::Provider(MultipartUploadError::Provider {
        error,
        abort_error: None,
    })
}

fn local_error(error: std::io::Error) -> MultipartFileUploadError {
    MultipartFileUploadError::Local {
        error,
        abort_error: None,
    }
}

async fn abort<T: MultipartUploader>(
    provider: &T,
    request: &MultipartUploadRequest,
    upload: &MultipartUpload,
    error: MultipartFileUploadError,
) -> MultipartFileUploadError {
    let abort_error = provider
        .abort_multipart_upload(&request.bucket, &request.key, upload)
        .await
        .err();
    match error {
        MultipartFileUploadError::Cancelled { .. } => {
            MultipartFileUploadError::Cancelled { abort_error }
        }
        MultipartFileUploadError::Provider(MultipartUploadError::Cancelled { .. }) => {
            MultipartFileUploadError::Cancelled { abort_error }
        }
        MultipartFileUploadError::Provider(MultipartUploadError::Provider { error, .. }) => {
            provider_error_with_abort(error, abort_error)
        }
        MultipartFileUploadError::Local { error, .. } => {
            MultipartFileUploadError::Local { error, abort_error }
        }
    }
}

fn provider_error_with_abort(
    error: ProviderError,
    abort_error: Option<ProviderError>,
) -> MultipartFileUploadError {
    MultipartFileUploadError::Provider(MultipartUploadError::Provider { error, abort_error })
}
