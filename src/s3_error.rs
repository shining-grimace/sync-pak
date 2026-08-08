use std::error::Error;

use aws_sdk_s3::error::{ConnectorError, ProvideErrorMetadata, SdkError};

use crate::provider_capabilities::{ProviderError, ProviderTransportError};

#[cfg(any(target_os = "android", test))]
use crate::endpoint_resolution_error::EndpointResolutionError;
#[cfg(any(target_os = "android", test))]
use crate::secure_connection_error::{SecureConnectionError, SecureConnectionErrorKind};

pub(crate) fn provider_error<E: ProvideErrorMetadata>(error: SdkError<E>) -> ProviderError {
    if let Some(code) = error.as_service_error().and_then(|value| value.code()) {
        return provider_error_code(code);
    }
    match error {
        SdkError::TimeoutError(_) => transport_error(ProviderTransportError::ConnectionTimedOut),
        SdkError::DispatchFailure(error) => match error.as_connector_error() {
            Some(error) => connector_error(error),
            None => transport_error(ProviderTransportError::Unexpected),
        },
        SdkError::ConstructionFailure(_) => ProviderError::InvalidRequest,
        SdkError::ResponseError(_) | SdkError::ServiceError(_) => ProviderError::Unexpected,
        _ => ProviderError::Unexpected,
    }
}

fn connector_error(error: &ConnectorError) -> ProviderError {
    if error.is_timeout() {
        return transport_error(ProviderTransportError::ConnectionTimedOut);
    }
    #[cfg(any(target_os = "android", test))]
    {
        if find_source::<EndpointResolutionError>(error).is_some() {
            return transport_error(ProviderTransportError::EndpointUnresolved);
        }
        if let Some(error) = find_source::<SecureConnectionError>(error) {
            return match error.kind() {
                SecureConnectionErrorKind::Certificate(error) => ProviderError::Certificate(*error),
                SecureConnectionErrorKind::ConnectionAborted => {
                    transport_error(ProviderTransportError::ConnectionAborted)
                }
                SecureConnectionErrorKind::ConnectionClosed => {
                    transport_error(ProviderTransportError::ConnectionClosed)
                }
                SecureConnectionErrorKind::ConnectionRefused => {
                    transport_error(ProviderTransportError::ConnectionRefused)
                }
                SecureConnectionErrorKind::ConnectionReset => {
                    transport_error(ProviderTransportError::ConnectionReset)
                }
                SecureConnectionErrorKind::ConnectionTimedOut => {
                    transport_error(ProviderTransportError::ConnectionTimedOut)
                }
                SecureConnectionErrorKind::HandshakeClosed => {
                    transport_error(ProviderTransportError::TlsHandshakeClosed)
                }
                SecureConnectionErrorKind::NativeCertificateVerifierFailed => {
                    transport_error(ProviderTransportError::NativeCertificateVerifierFailed)
                }
                SecureConnectionErrorKind::NetworkPermissionDenied => {
                    transport_error(ProviderTransportError::NetworkPermissionDenied)
                }
                SecureConnectionErrorKind::NetworkUnreachable => {
                    transport_error(ProviderTransportError::NetworkUnreachable)
                }
                SecureConnectionErrorKind::Opaque(detail) => {
                    transport_error(ProviderTransportError::OpaqueTlsIo(detail.clone()))
                }
                SecureConnectionErrorKind::TrustStoreUnavailable => {
                    transport_error(ProviderTransportError::TrustStoreUnavailable)
                }
            };
        }
    }
    #[cfg(target_os = "android")]
    if let Some(error) = find_source::<rustls::Error>(error) {
        return crate::s3_tls_error::classify(error);
    }
    if error.is_user() {
        return ProviderError::InvalidRequest;
    }
    if let Some(error) = deepest_source::<std::io::Error>(error) {
        return io_error(error);
    }
    transport_error(ProviderTransportError::Unexpected)
}

fn io_error(error: &std::io::Error) -> ProviderError {
    use std::io::ErrorKind;

    let error = match error.kind() {
        ErrorKind::ConnectionAborted => ProviderTransportError::ConnectionAborted,
        ErrorKind::BrokenPipe | ErrorKind::NotConnected | ErrorKind::UnexpectedEof => {
            ProviderTransportError::ConnectionClosed
        }
        ErrorKind::ConnectionRefused => ProviderTransportError::ConnectionRefused,
        ErrorKind::ConnectionReset => ProviderTransportError::ConnectionReset,
        ErrorKind::HostUnreachable | ErrorKind::NetworkUnreachable => {
            ProviderTransportError::NetworkUnreachable
        }
        ErrorKind::InvalidData => ProviderTransportError::InvalidTransportData,
        ErrorKind::AddrInUse | ErrorKind::AddrNotAvailable => {
            ProviderTransportError::LocalAddressUnavailable
        }
        ErrorKind::PermissionDenied => ProviderTransportError::NetworkPermissionDenied,
        ErrorKind::TimedOut => ProviderTransportError::ConnectionTimedOut,
        _ => ProviderTransportError::ConnectionFailed,
    };
    transport_error(error)
}

fn transport_error(error: ProviderTransportError) -> ProviderError {
    ProviderError::Transport(error)
}

#[cfg(any(target_os = "android", test))]
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

fn deepest_source<'a, E: Error + 'static>(error: &'a (dyn Error + 'static)) -> Option<&'a E> {
    let mut found = None;
    let mut current = Some(error);
    while let Some(error) = current {
        if let Some(source) = error.downcast_ref::<E>() {
            found = Some(source);
        }
        current = error.source();
    }
    found
}

fn provider_error_code(code: &str) -> ProviderError {
    match code {
        "AccessDenied" | "NotEntitled" | "ObjectLockedByBucketPolicy" => {
            ProviderError::PermissionDenied
        }
        "Unauthorized"
        | "InvalidAccessKeyId"
        | "InvalidToken"
        | "ExpiredToken"
        | "SignatureDoesNotMatch" => ProviderError::Authentication,
        "NoSuchBucket" | "NoSuchKey" | "NoSuchUpload" | "NotFound" => ProviderError::NotFound,
        "InternalError" | "ServiceUnavailable" | "SlowDown" | "TooManyRequests"
        | "RequestTimeout" => ProviderError::Unavailable,
        "AuthorizationHeaderMalformed"
        | "BadDigest"
        | "EntityTooLarge"
        | "EntityTooSmall"
        | "ExpiredRequest"
        | "IncompleteBody"
        | "InvalidArgument"
        | "InvalidBucketName"
        | "InvalidDigest"
        | "InvalidObjectName"
        | "InvalidPart"
        | "InvalidPartOrder"
        | "InvalidRange"
        | "InvalidRequest"
        | "MissingContentLength"
        | "PreconditionFailed"
        | "UnsupportedArgument"
        | "UnsupportedSignature" => ProviderError::InvalidRequest,
        "RequestTimeTooSkewed" => ProviderError::ClockSkew,
        _ => ProviderError::Unexpected,
    }
}

#[cfg(test)]
#[path = "s3_error_tests.rs"]
mod tests;
