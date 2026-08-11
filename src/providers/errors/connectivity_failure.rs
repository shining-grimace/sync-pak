use std::borrow::Cow;

use crate::providers::capabilities::{ProviderCertificateError, ProviderTransportError};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ProviderConnectivityFailure {
    Certificate(ProviderCertificateError),
    Transport(ProviderTransportError),
}

impl From<ProviderCertificateError> for ProviderConnectivityFailure {
    fn from(error: ProviderCertificateError) -> Self {
        Self::Certificate(error)
    }
}

impl From<ProviderTransportError> for ProviderConnectivityFailure {
    fn from(error: ProviderTransportError) -> Self {
        Self::Transport(error)
    }
}

impl ProviderConnectivityFailure {
    pub(crate) fn diagnostic(&self) -> Cow<'static, str> {
        match self {
            Self::Certificate(ProviderCertificateError::Expired) => {
                "provider TLS certificate has expired"
            }
            Self::Certificate(ProviderCertificateError::Invalid) => {
                "provider TLS certificate is invalid"
            }
            Self::Certificate(ProviderCertificateError::NameMismatch) => {
                "provider TLS certificate does not match the endpoint name"
            }
            Self::Certificate(ProviderCertificateError::NotYetValid) => {
                "provider TLS certificate is not yet valid"
            }
            Self::Certificate(ProviderCertificateError::Revoked) => {
                "provider TLS certificate has been revoked"
            }
            Self::Certificate(ProviderCertificateError::Untrusted) => {
                "system does not trust the provider TLS certificate chain"
            }
            Self::Transport(ProviderTransportError::ConnectionAborted) => {
                "provider connection was aborted before an HTTP response"
            }
            Self::Transport(ProviderTransportError::ConnectionClosed) => {
                "provider connection closed before an HTTP response"
            }
            Self::Transport(ProviderTransportError::ConnectionFailed) => {
                "provider connection failed before an HTTP response"
            }
            Self::Transport(ProviderTransportError::ConnectionRefused) => {
                "provider endpoint refused the connection"
            }
            Self::Transport(ProviderTransportError::ConnectionReset) => {
                "provider connection was reset before an HTTP response"
            }
            Self::Transport(ProviderTransportError::ConnectionTimedOut) => {
                "provider connection timed out before an HTTP response"
            }
            Self::Transport(ProviderTransportError::EndpointUnresolved) => {
                "provider endpoint name could not be resolved"
            }
            Self::Transport(ProviderTransportError::InvalidTransportData) => {
                "provider endpoint returned invalid secure HTTP data"
            }
            Self::Transport(ProviderTransportError::LocalAddressUnavailable) => {
                "device could not allocate a local network address"
            }
            Self::Transport(ProviderTransportError::NativeCertificateVerifierFailed) => {
                "Android native certificate verifier call failed"
            }
            Self::Transport(ProviderTransportError::NetworkPermissionDenied) => {
                "operating system denied the provider network connection"
            }
            Self::Transport(ProviderTransportError::NetworkUnreachable) => {
                "device has no network route to the provider endpoint"
            }
            Self::Transport(ProviderTransportError::OpaqueTlsIo(detail)) => {
                return Cow::Owned(format!(
                    "TLS handshake failed with an unclassified socket I/O error: {}",
                    detail.as_str()
                ));
            }
            Self::Transport(ProviderTransportError::RustlsGeneralFailed) => {
                "rustls reported an unclassified general TLS error"
            }
            Self::Transport(ProviderTransportError::SecureRandomUnavailable) => {
                "device secure random source was unavailable during TLS setup"
            }
            Self::Transport(ProviderTransportError::TlsAlertReceived) => {
                "provider rejected the TLS handshake with an alert"
            }
            Self::Transport(ProviderTransportError::TlsConfigurationFailed) => {
                "SyncPak TLS client configuration failed"
            }
            Self::Transport(ProviderTransportError::TlsDeviceTimeUnavailable) => {
                "device time was unavailable to the TLS verifier"
            }
            Self::Transport(ProviderTransportError::TlsHandshakeClosed) => {
                "provider TLS handshake closed before completion"
            }
            Self::Transport(ProviderTransportError::TlsProtocolFailed) => {
                "provider TLS protocol negotiation failed"
            }
            Self::Transport(ProviderTransportError::TrustStoreUnavailable) => {
                "Android system certificate trust store is unavailable"
            }
            Self::Transport(ProviderTransportError::Unexpected) => {
                "provider transport failed before an HTTP response"
            }
        }
        .into()
    }

    pub(crate) fn message(&self) -> &'static str {
        match self {
            Self::Certificate(ProviderCertificateError::Expired) => {
                "The provider's security certificate has expired. These credentials were not checked. If the device date is correct, this is a provider certificate problem."
            }
            Self::Certificate(ProviderCertificateError::Invalid) => {
                "The system rejected the provider's security certificate. These credentials were not checked. Check the endpoint; if it is correct, open Diagnostics and report this error."
            }
            Self::Certificate(ProviderCertificateError::NameMismatch) => {
                "The provider's security certificate does not match the configured endpoint. Check the account ID or custom endpoint. These credentials were not checked."
            }
            Self::Certificate(ProviderCertificateError::NotYetValid) => {
                "The provider's security certificate is not yet valid. Enable automatic date and time. If the device date is correct, this is a provider certificate problem."
            }
            Self::Certificate(ProviderCertificateError::Revoked) => {
                "The system rejected a revoked provider security certificate. This is a provider certificate problem, not a credentials or bucket configuration error."
            }
            Self::Certificate(ProviderCertificateError::Untrusted) => {
                "The system does not trust the provider's security certificate chain. Check the endpoint. If it is correct, this is a certificate or network-interception problem; the credentials were not checked."
            }
            Self::Transport(ProviderTransportError::ConnectionAborted) => {
                "The provider connection was aborted before a response arrived. No credentials or settings rejection was received. This usually indicates a provider, network, VPN, or firewall interruption."
            }
            Self::Transport(ProviderTransportError::ConnectionClosed) => {
                "The endpoint closed the connection without an HTTP response. Check the endpoint; if it is correct, this is a provider or network-interception problem rather than a credentials rejection."
            }
            Self::Transport(ProviderTransportError::ConnectionFailed) => {
                "The connection ended before the provider returned a response. This is not a credentials or bucket rejection. Check the endpoint and network; open Diagnostics if both are correct."
            }
            Self::Transport(ProviderTransportError::ConnectionRefused) => {
                "The configured endpoint resolved but refused the connection. The credentials were not checked. Check the endpoint; if it is correct, the provider service is not accepting connections."
            }
            Self::Transport(ProviderTransportError::ConnectionReset) => {
                "The connection was reset before the provider returned a response. No credentials or settings rejection was received. This usually indicates the provider or an intervening network device closed it."
            }
            Self::Transport(ProviderTransportError::ConnectionTimedOut) => {
                "The provider did not respond before the connection timed out. No credentials or settings rejection was received. Check the endpoint, device network, and provider availability."
            }
            Self::Transport(ProviderTransportError::EndpointUnresolved) => {
                "This device could not resolve the configured provider endpoint. Check the account ID or custom endpoint. The credentials were not checked."
            }
            Self::Transport(ProviderTransportError::InvalidTransportData) => {
                "The endpoint returned data that was not valid for a secure HTTP connection. Check the endpoint; if it is correct, this is a provider or network-interception problem, not a credentials rejection."
            }
            Self::Transport(ProviderTransportError::LocalAddressUnavailable) => {
                "This device could not allocate a local address for the connection. This is a device network, VPN, or operating-system problem, not a provider credentials error."
            }
            Self::Transport(ProviderTransportError::NativeCertificateVerifierFailed) => {
                "SyncPak could not call Android's native certificate verifier. This is an Android integration failure, not a provider settings or credentials error. Open Diagnostics and report it."
            }
            Self::Transport(ProviderTransportError::NetworkPermissionDenied) => {
                "Android denied the network connection at the operating-system level. This is a device permission or network-policy problem, not a provider configuration rejection."
            }
            Self::Transport(ProviderTransportError::NetworkUnreachable) => {
                "This device has no network route to the provider endpoint. This is a device network problem, not a credentials or bucket configuration error."
            }
            Self::Transport(ProviderTransportError::OpaqueTlsIo(_)) => {
                "The TLS handshake failed with a socket error whose inner category was hidden by the HTTP connector. This is not a credentials rejection. Open Diagnostics and report this implementation failure."
            }
            Self::Transport(ProviderTransportError::RustlsGeneralFailed) => {
                "rustls reported a general TLS failure that SyncPak could not classify further. This is not a credentials rejection. Open Diagnostics and report this implementation failure."
            }
            Self::Transport(ProviderTransportError::SecureRandomUnavailable) => {
                "Android could not provide secure random data required for TLS. This is a device integration failure, not a provider configuration or credentials error."
            }
            Self::Transport(ProviderTransportError::TlsAlertReceived) => {
                "The provider or an intervening network device rejected the TLS handshake. The credentials were not checked. If the endpoint works in a browser, open Diagnostics and report this compatibility error."
            }
            Self::Transport(ProviderTransportError::TlsConfigurationFailed) => {
                "SyncPak could not construct its TLS client configuration. This is an implementation failure, not a provider settings or credentials error. Open Diagnostics and report it."
            }
            Self::Transport(ProviderTransportError::TlsDeviceTimeUnavailable) => {
                "rustls could not read the device time required to verify the provider certificate. This is a device integration failure, not a credentials error."
            }
            Self::Transport(ProviderTransportError::TlsHandshakeClosed) => {
                "The secure connection closed before the TLS handshake completed. The credentials were not checked. If this endpoint works in a browser, this indicates a SyncPak TLS compatibility problem."
            }
            Self::Transport(ProviderTransportError::TlsProtocolFailed) => {
                "SyncPak and the endpoint could not complete TLS protocol negotiation. Check the endpoint; if it works in a browser, open Diagnostics and report this compatibility error."
            }
            Self::Transport(ProviderTransportError::TrustStoreUnavailable) => {
                "Android did not provide a usable system certificate trust store. This is a device integration problem, not a provider configuration error. Open Diagnostics and report it."
            }
            Self::Transport(ProviderTransportError::Unexpected) => {
                "The network transport failed before the provider returned a response. No credentials or settings rejection was received. Open Diagnostics and report this technical error."
            }
        }
    }
}

#[cfg(test)]
mod tests;
