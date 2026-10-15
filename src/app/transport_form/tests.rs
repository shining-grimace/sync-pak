use super::{fields, preview};
use crate::configuration::{ProviderKind, TransportSettings, lists};

#[test]
fn inferred_b2_region_replaces_a_stale_manual_value() {
    let settings = fields(
        ProviderKind::BackblazeB2,
        " s3.us-west-004.backblazeb2.com ",
        "old-region",
    );
    assert_eq!(settings.region.as_deref(), Some("us-west-004"));
    assert!(settings.resolve(ProviderKind::BackblazeB2, None).is_ok());
    let custom = fields(ProviderKind::BackblazeB2, "proxy.example", "us-west-004");
    assert_eq!(custom.region.as_deref(), Some("us-west-004"));
}

#[test]
fn r2_hides_and_discards_region_but_keeps_an_explicit_endpoint() {
    let settings = fields(
        ProviderKind::CloudflareR2,
        "account.eu.r2.cloudflarestorage.com",
        "eu",
    );
    assert!(settings.region.is_none());
    let mut config = lists::tests::config();
    config.providers[0].kind = ProviderKind::CloudflareR2;
    let summary = preview(&config.providers[0], "photos", Some(&settings)).summary;
    assert!(summary.contains("account.eu.r2.cloudflarestorage.com"));
    assert!(!summary.contains("Region:"));
    assert_eq!(
        settings
            .resolve(ProviderKind::CloudflareR2, None)
            .unwrap()
            .region
            .as_deref(),
        Some("auto")
    );
}

#[test]
fn summary_reflects_bucket_inheritance_and_explicit_standard_endpoints() {
    let mut config = lists::tests::config();
    let provider = &mut config.providers[0];
    provider.options.default_bucket_transport = Some(TransportSettings {
        endpoint: Some("https://private.example".into()),
        region: Some("eu-west-1".into()),
    });
    assert!(
        preview(provider, "photos-backup", None)
            .summary
            .contains("private.example")
    );
    assert!(
        !preview(provider, "other-bucket", None)
            .summary
            .contains("private.example")
    );
    let settings = fields(ProviderKind::AwsS3, "", "ap-southeast-2");
    let summary = preview(provider, "photos-backup", Some(&settings)).summary;
    assert!(summary.contains("AWS standard endpoint"));
    assert!(summary.contains("ap-southeast-2"));
    assert!(!summary.contains("private.example"));
}

#[test]
fn missing_connection_region_is_reported_before_network_access() {
    let config = lists::tests::config();
    let settings = TransportSettings::default();
    let summary = preview(&config.providers[0], "photos-backup", Some(&settings)).summary;
    assert!(summary.contains("Choose the AWS region"));
}
