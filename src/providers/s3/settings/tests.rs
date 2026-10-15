use super::S3Settings;
use crate::configuration::{
    CredentialReference, ProviderConfig, ProviderId, ProviderKind, ProviderOptions,
};

#[test]
fn r2_uses_its_account_endpoint_and_auto_region() {
    let settings = S3Settings::from_provider(&provider(
        ProviderKind::CloudflareR2,
        Some("account"),
        None,
        None,
    ))
    .unwrap();

    assert_eq!(
        settings.endpoint.as_deref(),
        Some("https://account.r2.cloudflarestorage.com")
    );
    assert_eq!(settings.region, "auto");
    assert!(settings.force_path_style);
    assert!(settings.request_checksums_when_required);
}

#[test]
fn b2_requires_an_explicit_regional_endpoint() {
    assert!(
        S3Settings::from_provider(&provider(
            ProviderKind::BackblazeB2,
            None,
            None,
            Some("us-west-001")
        ))
        .is_err()
    );
}

#[test]
fn saved_custom_endpoints_are_normalized_for_every_provider() {
    for kind in [
        ProviderKind::AwsS3,
        ProviderKind::BackblazeB2,
        ProviderKind::CloudflareR2,
    ] {
        let settings = S3Settings::from_provider(&provider(
            kind,
            None,
            Some(" s3.us-west-004.backblazeb2.com "),
            Some("us-west-004"),
        ))
        .unwrap();
        assert_eq!(
            settings.endpoint.as_deref(),
            Some("https://s3.us-west-004.backblazeb2.com")
        );
    }
}

fn provider(
    kind: ProviderKind,
    account_id: Option<&str>,
    endpoint: Option<&str>,
    region: Option<&str>,
) -> ProviderConfig {
    let id = ProviderId::new();
    ProviderConfig {
        credential_reference: CredentialReference {
            provider_id: id.clone(),
        },
        id,
        name: "Test".to_owned(),
        kind,
        options: ProviderOptions {
            default_bucket_transport: None,
            account_id: account_id.map(ToOwned::to_owned),
            default_bucket: None,
            endpoint: endpoint.map(ToOwned::to_owned),
            region: region.map(ToOwned::to_owned),
        },
        verified: false,
    }
}
#[test]
fn b2_infers_region_and_rejects_conflicting_region() {
    let mut provider = provider(
        ProviderKind::BackblazeB2,
        None,
        Some("s3.us-west-004.backblazeb2.com"),
        None,
    );
    assert_eq!(
        S3Settings::from_provider(&provider).unwrap().region,
        "us-west-004"
    );
    provider.options.region = Some("us-east-005".into());
    assert!(S3Settings::from_provider(&provider).is_err());
}

#[test]
fn custom_b2_host_requires_explicit_region() {
    let mut provider = provider(
        ProviderKind::BackblazeB2,
        None,
        Some("https://proxy.example"),
        None,
    );
    assert!(S3Settings::from_provider(&provider).is_err());
    provider.options.region = Some("us-west-004".into());
    assert_eq!(
        S3Settings::from_provider(&provider).unwrap().region,
        "us-west-004"
    );
}

#[test]
fn aws_connection_can_select_a_different_region_and_standard_endpoint() {
    let provider = provider(
        ProviderKind::AwsS3,
        None,
        Some("https://private.example"),
        Some("us-east-1"),
    );
    let connection = crate::configuration::TransportSettings {
        endpoint: None,
        region: Some("ap-southeast-2".into()),
    };
    let settings = S3Settings::for_bucket(&provider, Some("photos"), Some(&connection)).unwrap();
    assert_eq!(settings.region, "ap-southeast-2");
    assert!(settings.endpoint.is_none());
    let missing_region = crate::configuration::TransportSettings::default();
    assert!(S3Settings::for_bucket(&provider, Some("photos"), Some(&missing_region)).is_err());
}

#[test]
fn r2_connection_can_select_a_jurisdiction_endpoint_with_auto_signing() {
    let provider = provider(ProviderKind::CloudflareR2, Some("account"), None, None);
    let connection = crate::configuration::TransportSettings {
        endpoint: Some("https://account.eu.r2.cloudflarestorage.com".into()),
        region: None,
    };
    let settings = S3Settings::for_bucket(&provider, Some("eu-photos"), Some(&connection)).unwrap();
    assert_eq!(settings.endpoint, connection.endpoint);
    assert_eq!(settings.region, "auto");
}

#[test]
fn default_bucket_settings_do_not_leak_to_other_connections() {
    let mut provider = provider(ProviderKind::AwsS3, None, None, Some("us-east-1"));
    provider.options.default_bucket = Some("photos".into());
    provider.options.default_bucket_transport = Some(crate::configuration::TransportSettings {
        endpoint: Some("https://bucket.example".into()),
        region: Some("eu-west-1".into()),
    });
    assert_eq!(
        S3Settings::from_provider(&provider).unwrap().region,
        "eu-west-1"
    );
    let other = S3Settings::for_bucket(&provider, Some("other"), None).unwrap();
    assert_eq!(other.region, "us-east-1");
    assert!(other.endpoint.is_none());
}
