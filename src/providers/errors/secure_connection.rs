use std::error::Error;

use crate::providers::{
    capabilities::{ProviderCertificateError, ProviderTransportError},
    errors::safe_transport_detail::SafeTransportDetail,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum SecureConnectionErrorKind {
    Certificate(ProviderCertificateError),
    ConnectionAborted,
    ConnectionClosed,
    ConnectionRefused,
    ConnectionReset,
    ConnectionTimedOut,
    HandshakeClosed,
    NativeCertificateVerifierFailed,
    NetworkPermissionDenied,
    NetworkUnreachable,
    Opaque(SafeTransportDetail),
    TrustStoreUnavailable,
}

impl SecureConnectionErrorKind {
    pub(crate) fn from_io_error(
        error: &std::io::Error,
        error_chain: &(dyn Error + 'static),
    ) -> Self {
        let message = error.to_string().to_ascii_lowercase();
        if let Some(error) = hidden_certificate_error(&message) {
            return Self::Certificate(error);
        }
        if message.contains("failed to call native verifier:") {
            return Self::NativeCertificateVerifierFailed;
        }
        if message.contains("no system trust stores available") {
            return Self::TrustStoreUnavailable;
        }
        if message == "tls handshake eof" {
            return Self::HandshakeClosed;
        }
        if message.contains("connection reset") || message.contains("reset by peer") {
            Self::ConnectionReset
        } else if message.contains("connection aborted") {
            Self::ConnectionAborted
        } else if message.contains("connection refused") {
            Self::ConnectionRefused
        } else if message.contains("timed out") || message.contains("timeout") {
            Self::ConnectionTimedOut
        } else if message.contains("network is unreachable")
            || message.contains("host is unreachable")
            || message.contains("no route to host")
        {
            Self::NetworkUnreachable
        } else if message.contains("permission denied") {
            Self::NetworkPermissionDenied
        } else if message.contains("broken pipe")
            || message.contains("connection closed")
            || message.contains("unexpected eof")
        {
            Self::ConnectionClosed
        } else {
            Self::Opaque(SafeTransportDetail::from_error_chain(error_chain))
        }
    }
}

fn hidden_certificate_error(message: &str) -> Option<ProviderCertificateError> {
    let detail = message
        .split_once("invalid peer certificate: ")?
        .1
        .trim_start();
    Some(if detail.starts_with("revoked") {
        ProviderCertificateError::Revoked
    } else if detail.starts_with("expired") || detail.starts_with("certificate expired") {
        ProviderCertificateError::Expired
    } else if detail.starts_with("notvalidyet") || detail.starts_with("certificate not valid yet") {
        ProviderCertificateError::NotYetValid
    } else if detail.starts_with("unknownissuer") {
        ProviderCertificateError::Untrusted
    } else if detail.starts_with("notvalidforname")
        || detail.starts_with("certificate not valid for name")
    {
        ProviderCertificateError::NameMismatch
    } else {
        ProviderCertificateError::Invalid
    })
}

pub(crate) fn general_tls_error(message: &str) -> ProviderTransportError {
    if message == "No system trust stores available" {
        ProviderTransportError::TrustStoreUnavailable
    } else if message.starts_with("failed to call native verifier:") {
        ProviderTransportError::NativeCertificateVerifierFailed
    } else {
        ProviderTransportError::RustlsGeneralFailed
    }
}

#[derive(Debug)]
pub(crate) struct SecureConnectionError {
    kind: SecureConnectionErrorKind,
    source: Box<dyn Error + Send + Sync + 'static>,
}

impl SecureConnectionError {
    pub(crate) fn new(
        kind: SecureConnectionErrorKind,
        source: Box<dyn Error + Send + Sync + 'static>,
    ) -> Self {
        Self { kind, source }
    }

    pub(crate) fn kind(&self) -> &SecureConnectionErrorKind {
        &self.kind
    }
}

impl std::fmt::Display for SecureConnectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("secure provider connection failed")
    }
}

impl Error for SecureConnectionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.source.as_ref())
    }
}

#[cfg(test)]
mod tests {
    use super::SecureConnectionErrorKind;
    use crate::providers::capabilities::ProviderCertificateError;

    #[test]
    fn identifies_tokio_rustls_handshake_eof_without_exposing_error_text() {
        let closed = std::io::Error::other("tls handshake eof");
        let other = std::io::Error::other("another secure connection error");

        assert_eq!(
            SecureConnectionErrorKind::from_io_error(&closed, &closed),
            SecureConnectionErrorKind::HandshakeClosed
        );
        assert!(matches!(
            SecureConnectionErrorKind::from_io_error(&other, &other),
            SecureConnectionErrorKind::Opaque(detail)
                if detail.as_str().contains("another secure connection error")
        ));
    }

    #[test]
    fn identifies_socket_failures_hidden_by_hyper_rustls() {
        for (message, expected) in [
            (
                "Connection reset by peer (os error 104)",
                SecureConnectionErrorKind::ConnectionReset,
            ),
            (
                "Broken pipe (os error 32)",
                SecureConnectionErrorKind::ConnectionClosed,
            ),
            (
                "Connection timed out (os error 110)",
                SecureConnectionErrorKind::ConnectionTimedOut,
            ),
            (
                "Network is unreachable (os error 101)",
                SecureConnectionErrorKind::NetworkUnreachable,
            ),
        ] {
            let source = std::io::Error::other(message);
            assert_eq!(
                SecureConnectionErrorKind::from_io_error(&source, &source),
                expected
            );
        }
    }

    #[test]
    fn recovers_certificate_failures_hidden_by_hyper_rustls() {
        for (message, expected) in [
            (
                "client error (Connect): invalid peer certificate: Revoked",
                ProviderCertificateError::Revoked,
            ),
            (
                "client error (Connect): invalid peer certificate: Expired",
                ProviderCertificateError::Expired,
            ),
            (
                "client error (Connect): invalid peer certificate: UnknownIssuer",
                ProviderCertificateError::Untrusted,
            ),
            (
                "client error (Connect): invalid peer certificate: NotValidForName",
                ProviderCertificateError::NameMismatch,
            ),
        ] {
            let source = std::io::Error::other(message);
            assert_eq!(
                SecureConnectionErrorKind::from_io_error(&source, &source),
                SecureConnectionErrorKind::Certificate(expected)
            );
        }
    }

    #[test]
    fn recovers_native_verifier_failures_hidden_by_hyper_rustls() {
        let native =
            std::io::Error::other("client error (Connect): failed to call native verifier: Error");
        let trust =
            std::io::Error::other("client error (Connect): No system trust stores available");

        assert_eq!(
            SecureConnectionErrorKind::from_io_error(&native, &native),
            SecureConnectionErrorKind::NativeCertificateVerifierFailed
        );
        assert_eq!(
            SecureConnectionErrorKind::from_io_error(&trust, &trust),
            SecureConnectionErrorKind::TrustStoreUnavailable
        );
    }
}
