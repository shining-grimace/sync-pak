use super::*;

#[test]
fn non_aws_providers_require_an_endpoint() {
    let result = ProbeConfig::from_values(|name| match name {
        "SYNCPAK_PROBE_PROVIDER" => Some("cloudflare-r2".to_owned()),
        "SYNCPAK_PROBE_ACCESS_KEY_ID"
        | "SYNCPAK_PROBE_BUCKET"
        | "SYNCPAK_PROBE_PREFIX"
        | "SYNCPAK_PROBE_SECRET_ACCESS_KEY" => Some("value".to_owned()),
        _ => None,
    });

    assert!(matches!(
        result,
        Err(ProbeError::MissingSetting("SYNCPAK_PROBE_ENDPOINT"))
    ));
}

#[test]
fn prefix_is_normalized() {
    let config = ProbeConfig::from_values(|name| match name {
        "SYNCPAK_PROBE_PROVIDER" => Some("aws-s3".to_owned()),
        "SYNCPAK_PROBE_ACCESS_KEY_ID"
        | "SYNCPAK_PROBE_BUCKET"
        | "SYNCPAK_PROBE_SECRET_ACCESS_KEY" => Some("value".to_owned()),
        "SYNCPAK_PROBE_PREFIX" => Some("/isolated-test/".to_owned()),
        _ => None,
    })
    .unwrap();

    assert_eq!(config.prefix, "isolated-test");
}

#[test]
fn bucket_listing_is_opt_in() {
    let config = ProbeConfig::from_values(|name| match name {
        "SYNCPAK_PROBE_PROVIDER" => Some("aws-s3".to_owned()),
        "SYNCPAK_PROBE_ACCESS_KEY_ID"
        | "SYNCPAK_PROBE_BUCKET"
        | "SYNCPAK_PROBE_PREFIX"
        | "SYNCPAK_PROBE_SECRET_ACCESS_KEY" => Some("value".to_owned()),
        "SYNCPAK_PROBE_CHECK_BUCKET_LISTING" => Some("true".to_owned()),
        _ => None,
    })
    .unwrap();

    assert!(config.check_bucket_listing);
}

#[test]
fn conformance_errors_name_the_safe_failure_phase() {
    let error = ProbeError::Conformance(crate::providers::conformance::ConformanceError {
        phase: crate::providers::conformance::ConformancePhase::ObjectMetadata,
        provider_error: crate::providers::capabilities::ProviderError::PermissionDenied,
    });

    assert_eq!(
        error.to_string(),
        "The provider probe object metadata failed: The provider did not allow this operation."
    );
}
