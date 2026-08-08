use std::io;

use aws_sdk_s3::error::ConnectorError;

use super::{connector_error, provider_error_code};
use crate::{
    endpoint_resolution_error::EndpointResolutionError,
    provider_capabilities::{ProviderCertificateError, ProviderError, ProviderTransportError},
    safe_transport_detail::SafeTransportDetail,
    secure_connection_error::{
        SecureConnectionError, SecureConnectionErrorKind, general_tls_error,
    },
};

#[test]
fn known_r2_and_s3_responses_have_safe_categories() {
    assert_eq!(
        provider_error_code("Unauthorized"),
        ProviderError::Authentication
    );
    assert_eq!(
        provider_error_code("InvalidRequest"),
        ProviderError::InvalidRequest
    );
    assert_eq!(
        provider_error_code("ServiceUnavailable"),
        ProviderError::Unavailable
    );
    assert_eq!(
        provider_error_code("RequestTimeTooSkewed"),
        ProviderError::ClockSkew
    );
}

#[test]
fn unknown_service_responses_are_not_reported_as_network_failures() {
    assert_eq!(
        provider_error_code("VendorSpecificError"),
        ProviderError::Unexpected
    );
}

#[test]
fn connector_failures_preserve_network_failure_modes() {
    let timeout = ConnectorError::timeout(Box::new(io::Error::from(io::ErrorKind::TimedOut)));
    let refused = ConnectorError::io(Box::new(io::Error::from(io::ErrorKind::ConnectionRefused)));
    let unreachable =
        ConnectorError::io(Box::new(io::Error::from(io::ErrorKind::NetworkUnreachable)));

    assert_eq!(
        connector_error(&timeout),
        ProviderError::Transport(ProviderTransportError::ConnectionTimedOut)
    );
    assert_eq!(
        connector_error(&refused),
        ProviderError::Transport(ProviderTransportError::ConnectionRefused)
    );
    assert_eq!(
        connector_error(&unreachable),
        ProviderError::Transport(ProviderTransportError::NetworkUnreachable)
    );
}

#[test]
fn classified_secure_connection_error_survives_sdk_wrapping() {
    let opaque = io::Error::other("TLS connection failed");
    let detail = SafeTransportDetail::from_error_chain(&opaque);
    let error = ConnectorError::io(Box::new(SecureConnectionError::new(
        SecureConnectionErrorKind::Opaque(detail.clone()),
        Box::new(opaque),
    )));

    assert_eq!(
        connector_error(&error),
        ProviderError::Transport(ProviderTransportError::OpaqueTlsIo(detail))
    );
}

#[test]
fn closed_tls_handshake_survives_sdk_wrapping() {
    let closed = Box::new(io::Error::other("tls handshake eof"));
    let error = ConnectorError::io(Box::new(SecureConnectionError::new(
        SecureConnectionErrorKind::HandshakeClosed,
        closed,
    )));

    assert_eq!(
        connector_error(&error),
        ProviderError::Transport(ProviderTransportError::TlsHandshakeClosed)
    );
}

#[test]
fn revoked_certificate_failure_survives_sdk_wrapping() {
    let source = Box::new(io::Error::other(
        "client error (Connect): invalid peer certificate: Revoked",
    ));
    let error = ConnectorError::io(Box::new(SecureConnectionError::new(
        SecureConnectionErrorKind::Certificate(ProviderCertificateError::Revoked),
        source,
    )));

    assert_eq!(
        connector_error(&error),
        ProviderError::Certificate(ProviderCertificateError::Revoked)
    );
}

#[test]
fn native_verifier_general_error_has_its_own_safe_category() {
    assert_eq!(
        general_tls_error("failed to call native verifier: redacted JNI details"),
        ProviderTransportError::NativeCertificateVerifierFailed
    );
    assert_eq!(
        general_tls_error("another rustls error"),
        ProviderTransportError::RustlsGeneralFailed
    );
}

#[test]
fn remaining_io_failures_have_specific_safe_categories() {
    for (kind, expected) in [
        (
            io::ErrorKind::ConnectionAborted,
            ProviderTransportError::ConnectionAborted,
        ),
        (
            io::ErrorKind::ConnectionReset,
            ProviderTransportError::ConnectionReset,
        ),
        (
            io::ErrorKind::UnexpectedEof,
            ProviderTransportError::ConnectionClosed,
        ),
        (
            io::ErrorKind::InvalidData,
            ProviderTransportError::InvalidTransportData,
        ),
        (
            io::ErrorKind::PermissionDenied,
            ProviderTransportError::NetworkPermissionDenied,
        ),
        (
            io::ErrorKind::AddrNotAvailable,
            ProviderTransportError::LocalAddressUnavailable,
        ),
    ] {
        let error = ConnectorError::io(Box::new(io::Error::from(kind)));
        assert_eq!(connector_error(&error), ProviderError::Transport(expected));
    }
}

#[test]
fn explicit_resolution_errors_are_not_collapsed_into_io_failures() {
    let resolution = EndpointResolutionError::new(io::Error::from(io::ErrorKind::NotFound));
    let error = ConnectorError::io(Box::new(resolution));

    assert_eq!(
        connector_error(&error),
        ProviderError::Transport(ProviderTransportError::EndpointUnresolved)
    );
}

#[test]
fn invalid_http_requests_are_not_reported_as_network_failures() {
    let error = ConnectorError::user(Box::new(io::Error::from(io::ErrorKind::InvalidInput)));

    assert_eq!(connector_error(&error), ProviderError::InvalidRequest);
}
