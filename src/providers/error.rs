use crate::providers::errors::safe_transport_detail::SafeTransportDetail;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[allow(
    dead_code,
    reason = "certificate failures are an exhaustive cross-platform provider taxonomy"
)]
pub enum ProviderCertificateError {
    Expired,
    Invalid,
    NameMismatch,
    NotYetValid,
    Revoked,
    Untrusted,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(
    dead_code,
    reason = "transport failures are an exhaustive cross-platform provider taxonomy"
)]
pub enum ProviderTransportError {
    ConnectionAborted,
    ConnectionClosed,
    ConnectionFailed,
    ConnectionRefused,
    ConnectionReset,
    ConnectionTimedOut,
    EndpointUnresolved,
    InvalidTransportData,
    LocalAddressUnavailable,
    NativeCertificateVerifierFailed,
    NetworkPermissionDenied,
    NetworkUnreachable,
    OpaqueTlsIo(SafeTransportDetail),
    RustlsGeneralFailed,
    SecureRandomUnavailable,
    TlsAlertReceived,
    TlsConfigurationFailed,
    TlsDeviceTimeUnavailable,
    TlsHandshakeClosed,
    TlsProtocolFailed,
    TrustStoreUnavailable,
    Unexpected,
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[allow(
    dead_code,
    reason = "provider failures include capabilities unavailable on some targets"
)]
pub enum ProviderError {
    Authentication,
    Certificate(ProviderCertificateError),
    ClockSkew,
    InvalidRequest,
    NotFound,
    PermissionDenied,
    Transport(ProviderTransportError),
    Unavailable,
    Unsupported,
    Unexpected,
}

impl ProviderError {
    pub(crate) fn is_retryable(&self) -> bool {
        match self {
            Self::Unavailable => true,
            Self::Transport(error) => matches!(
                error,
                ProviderTransportError::ConnectionAborted
                    | ProviderTransportError::ConnectionClosed
                    | ProviderTransportError::ConnectionFailed
                    | ProviderTransportError::ConnectionRefused
                    | ProviderTransportError::ConnectionReset
                    | ProviderTransportError::ConnectionTimedOut
                    | ProviderTransportError::EndpointUnresolved
                    | ProviderTransportError::NetworkUnreachable
                    | ProviderTransportError::TlsHandshakeClosed
            ),
            _ => false,
        }
    }
}

impl std::fmt::Display for ProviderError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Authentication => "The provider rejected the saved credentials.",
            Self::Certificate(_) => "The provider certificate was rejected.",
            Self::ClockSkew => {
                "This device's clock differs too much from the provider. Enable automatic date and time, then retry."
            }
            Self::InvalidRequest => "The provider request is not valid.",
            Self::NotFound => "The requested provider resource was not found.",
            Self::PermissionDenied => "The provider did not allow this operation.",
            Self::Transport(_) => "The provider connection failed before an HTTP response.",
            Self::Unavailable => "The provider reported that it is unavailable.",
            Self::Unsupported => "The provider does not support this operation.",
            Self::Unexpected => "The provider could not complete the operation.",
        })
    }
}

impl std::error::Error for ProviderError {}

#[cfg(test)]
mod tests {
    use super::{ProviderError, ProviderTransportError};

    #[test]
    fn retries_only_transient_transport_and_service_failures() {
        assert!(ProviderError::Unavailable.is_retryable());
        assert!(
            ProviderError::Transport(ProviderTransportError::ConnectionTimedOut).is_retryable()
        );
        assert!(!ProviderError::Authentication.is_retryable());
        assert!(
            !ProviderError::Transport(ProviderTransportError::RustlsGeneralFailed).is_retryable()
        );
    }
}
