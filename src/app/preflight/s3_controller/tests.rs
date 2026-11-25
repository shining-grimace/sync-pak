use super::PreflightFailure;
use crate::capabilities::{CapabilityError, CredentialBackend};

#[test]
fn run_failures_identify_the_failing_connection_side() {
    let local = PreflightFailure::LocalInventory.message(CredentialBackend::Gnome);
    let remote = PreflightFailure::RemoteInventory.message(CredentialBackend::Gnome);

    assert!(local.contains("local folder"));
    assert!(!local.contains("bucket"));
    assert!(remote.contains("cloud bucket or remote folder"));
    assert!(!remote.contains("local folder"));
}

#[test]
fn credential_errors_name_the_saved_store_and_distinguish_the_failure() {
    for (error, reason) in [
        (
            CapabilityError::ServiceMissing,
            "unavailable in this session",
        ),
        (CapabilityError::Locked, "is locked"),
        (
            CapabilityError::NotFound,
            "does not have a saved credential entry",
        ),
    ] {
        let message = PreflightFailure::Credentials(Some(error)).message(CredentialBackend::Gnome);
        assert!(message.contains("GNOME Keyring"));
        assert!(message.contains(reason));
    }
}
