use super::VerificationFailure;
use crate::provider_capabilities::ProviderError;

#[test]
fn endpoint_failures_name_only_the_side_that_needs_attention() {
    let local = VerificationFailure::LocalFolderMissing.message();
    let remote = VerificationFailure::RemoteFolderMissing.message();

    assert!(local.contains("local folder"));
    assert!(!local.contains("bucket"));
    assert!(remote.contains("cloud bucket or remote folder"));
    assert!(!remote.contains("local folder"));
}

#[test]
fn missing_provider_resources_are_cloud_path_failures() {
    assert_eq!(
        VerificationFailure::from(ProviderError::NotFound),
        VerificationFailure::RemoteFolderMissing
    );
}
