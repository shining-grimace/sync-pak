use super::PreflightFailure;

#[test]
fn run_failures_identify_the_failing_connection_side() {
    let local = PreflightFailure::LocalInventory.message();
    let remote = PreflightFailure::RemoteInventory.message();

    assert!(local.contains("local folder"));
    assert!(!local.contains("bucket"));
    assert!(remote.contains("cloud bucket or remote folder"));
    assert!(!remote.contains("local folder"));
}
