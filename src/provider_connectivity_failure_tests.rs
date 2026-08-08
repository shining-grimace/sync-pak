use super::ProviderConnectivityFailure;
use crate::{
    provider_capabilities::{ProviderCertificateError, ProviderTransportError},
    safe_transport_detail::SafeTransportDetail,
};

#[test]
fn endpoint_resolution_names_configuration_and_credential_boundaries() {
    let failure =
        ProviderConnectivityFailure::Transport(ProviderTransportError::EndpointUnresolved);

    assert!(failure.message().contains("account ID or custom endpoint"));
    assert!(failure.message().contains("credentials were not checked"));
}

#[test]
fn certificate_name_mismatch_points_to_the_endpoint() {
    let failure = ProviderConnectivityFailure::Certificate(ProviderCertificateError::NameMismatch);

    assert!(failure.diagnostic().contains("does not match"));
    assert!(failure.message().contains("configured endpoint"));
}

#[test]
fn confirmed_revocation_is_a_provider_certificate_problem() {
    let failure = ProviderConnectivityFailure::Certificate(ProviderCertificateError::Revoked);

    assert!(failure.diagnostic().contains("has been revoked"));
    assert!(failure.message().contains("provider certificate problem"));
    assert!(failure.message().contains("not a credentials"));
}

#[test]
fn unclassified_rustls_failure_is_identified_as_an_implementation_error() {
    let failure =
        ProviderConnectivityFailure::Transport(ProviderTransportError::RustlsGeneralFailed);

    assert!(failure.message().contains("implementation failure"));
}

#[test]
fn opaque_tls_io_diagnostic_includes_its_safe_cause() {
    let source = std::io::Error::other("handshake syscall returned error 71");
    let detail = SafeTransportDetail::from_error_chain(&source);
    let failure =
        ProviderConnectivityFailure::Transport(ProviderTransportError::OpaqueTlsIo(detail));

    assert!(
        failure
            .diagnostic()
            .contains("handshake syscall returned error 71")
    );
    assert!(failure.message().contains("not a credentials rejection"));
}

#[test]
fn secure_connection_subcategories_have_distinct_diagnostics() {
    let handshake =
        ProviderConnectivityFailure::Transport(ProviderTransportError::TlsHandshakeClosed);
    let native = ProviderConnectivityFailure::Transport(
        ProviderTransportError::NativeCertificateVerifierFailed,
    );
    let alert = ProviderConnectivityFailure::Transport(ProviderTransportError::TlsAlertReceived);
    let protocol =
        ProviderConnectivityFailure::Transport(ProviderTransportError::TlsProtocolFailed);

    assert!(handshake.diagnostic().contains("closed"));
    assert!(native.diagnostic().contains("native certificate verifier"));
    assert!(alert.diagnostic().contains("alert"));
    assert!(protocol.diagnostic().contains("protocol negotiation"));
}
