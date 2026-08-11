use std::{error::Error, fmt};

use crate::{
    operations::cancellation::Cancelled, operations::transfer::multipart::MultipartUploadError,
    providers::capabilities::ProviderError,
};

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
