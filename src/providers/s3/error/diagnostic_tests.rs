use aws_sdk_s3::error::ConnectorError;

use super::connector_error;
use crate::providers::{
    capabilities::{ProviderError, ProviderTransportError},
    verification::failure::VerificationFailure,
};

#[derive(Debug)]
struct ProtocolFailure;

impl std::fmt::Display for ProtocolFailure {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(
            "invalid URL scheme for https://private.example/path?token=secret ABCD1234EFGH5678IJKL",
        )
    }
}

impl std::error::Error for ProtocolFailure {}

#[test]
fn unclassified_connector_causes_reach_diagnostics_without_endpoint_or_key() {
    let error = ConnectorError::other(Box::new(ProtocolFailure), None);
    let mapped = connector_error(&error);
    assert!(matches!(
        mapped,
        ProviderError::Transport(ProviderTransportError::OpaqueTransport(_))
    ));
    let failure = VerificationFailure::from(mapped);
    let detail = failure.diagnostic();
    assert!(detail.contains("invalid URL scheme"));
    assert!(!detail.contains("private.example"));
    assert!(!detail.contains("secret"));
    assert!(!detail.contains("ABCD1234EFGH5678IJKL"));
    assert!(detail.contains("[redacted]"));
}
