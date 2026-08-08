#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::error::Error;

use aws_smithy_runtime_api::{box_error::BoxError, client::result::ConnectorError};

use crate::{
    android_http_timeout::HttpTimeout,
    secure_connection_error::{SecureConnectionError, SecureConnectionErrorKind},
};

pub(crate) fn classify(error: hyper_util::client::legacy::Error) -> ConnectorError {
    let is_connect = error.is_connect();
    let error: BoxError = Box::new(error);
    let secure_error = is_connect
        .then(|| opaque_io_error_kind(error.as_ref()))
        .flatten();
    if find_source::<HttpTimeout>(error.as_ref()).is_some() {
        ConnectorError::timeout(error)
    } else if let Some(kind) = secure_error {
        ConnectorError::io(Box::new(SecureConnectionError::new(kind, error)))
    } else if is_connect || find_source::<std::io::Error>(error.as_ref()).is_some() {
        ConnectorError::io(error)
    } else {
        ConnectorError::other(error, None)
    }
}

fn opaque_io_error_kind(error: &(dyn Error + 'static)) -> Option<SecureConnectionErrorKind> {
    if find_source::<rustls::Error>(error).is_some() {
        return None;
    }
    find_source::<std::io::Error>(error)
        .filter(|error| error.kind() == std::io::ErrorKind::Other)
        .map(|io_error| SecureConnectionErrorKind::from_io_error(io_error, error))
}

fn find_source<'a, E: Error + 'static>(error: &'a (dyn Error + 'static)) -> Option<&'a E> {
    let mut current = Some(error);
    while let Some(error) = current {
        if let Some(found) = error.downcast_ref::<E>() {
            return Some(found);
        }
        current = error.source();
    }
    None
}
