use std::{error::Error, fmt, io::Read, path::Path};

use crate::{
    cancellation::{CancellationToken, Cancelled},
    multipart_upload::MultipartUploadError,
    provider_capabilities::{
        MultipartUpload, MultipartUploadRequest, MultipartUploader, ProviderError,
    },
};

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
    if part_size == 0 {
        return Err(local_error(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "multipart part size must not be zero",
        )));
    }
    let mut file = std::fs::File::open(source).map_err(local_error)?;
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
        let read = match file.read(&mut buffer) {
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

#[derive(Debug)]
pub enum MultipartFileUploadError {
    Cancelled {
        abort_error: Option<ProviderError>,
    },
    Provider(MultipartUploadError),
    Local {
        error: std::io::Error,
        abort_error: Option<ProviderError>,
    },
}

impl fmt::Display for MultipartFileUploadError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled { abort_error } => match abort_error {
                Some(abort_error) => write!(
                    formatter,
                    "multipart upload was cancelled; cleanup also failed: {abort_error}"
                ),
                None => formatter.write_str("multipart upload was cancelled"),
            },
            Self::Provider(error) => error.fmt(formatter),
            Self::Local { error, abort_error } => match abort_error {
                Some(abort_error) => write!(
                    formatter,
                    "could not read multipart upload source: {error}; cleanup also failed: {abort_error}"
                ),
                None => write!(formatter, "could not read multipart upload source: {error}"),
            },
        }
    }
}

impl Error for MultipartFileUploadError {}

impl From<Cancelled> for MultipartFileUploadError {
    fn from(_: Cancelled) -> Self {
        Self::Cancelled { abort_error: None }
    }
}
