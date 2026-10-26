mod provider_order;

use super::{ProviderFields, connection, provider};
use crate::{configuration::ProviderKind, validation::NAME_MAX_LENGTH};

fn valid_provider(name: &str) -> Result<(), String> {
    provider(ProviderFields {
        name,
        access_key_id: "access",
        secret_access_key: "secret",
        kind: ProviderKind::AwsS3,
        account_id: "",
        region: "region",
        default_bucket: "",
        endpoint: "",
    })
}

#[test]
fn provider_credentials_are_required() {
    assert_eq!(
        provider(ProviderFields {
            name: "Provider",
            access_key_id: "",
            secret_access_key: "secret",
            kind: ProviderKind::AwsS3,
            account_id: "",
            region: "region",
            default_bucket: "",
            endpoint: "",
        }),
        Err("Access key ID is required.".to_owned())
    );
    assert_eq!(
        provider(ProviderFields {
            name: "Provider",
            access_key_id: "access",
            secret_access_key: "  ",
            kind: ProviderKind::AwsS3,
            account_id: "",
            region: "region",
            default_bucket: "",
            endpoint: "",
        }),
        Err("Secret access key is required.".to_owned())
    );
}

#[test]
fn r2_needs_an_account_while_routing_defaults_can_be_configured_later() {
    assert_eq!(
        provider(ProviderFields {
            name: "Provider",
            access_key_id: "access",
            secret_access_key: "secret",
            kind: ProviderKind::CloudflareR2,
            account_id: "",
            region: "",
            default_bucket: "",
            endpoint: "",
        }),
        Err("Account ID is required.".to_owned())
    );
    assert_eq!(
        provider(ProviderFields {
            name: "Provider",
            access_key_id: "access",
            secret_access_key: "secret",
            kind: ProviderKind::AwsS3,
            account_id: "",
            region: "",
            default_bucket: "",
            endpoint: "",
        }),
        Ok(())
    );
    assert_eq!(
        provider(ProviderFields {
            name: "Provider",
            access_key_id: "access",
            secret_access_key: "secret",
            kind: ProviderKind::BackblazeB2,
            account_id: "",
            region: "",
            default_bucket: "",
            endpoint: "",
        }),
        Ok(())
    );
}

#[test]
fn optional_bucket_cannot_have_outer_whitespace() {
    assert_eq!(
        provider(ProviderFields {
            name: "Provider",
            access_key_id: "access",
            secret_access_key: "secret",
            kind: ProviderKind::AwsS3,
            account_id: "",
            region: "region",
            default_bucket: " bucket",
            endpoint: "",
        }),
        Err("Bucket name cannot begin or end with whitespace.".to_owned())
    );
}

#[test]
fn connection_fields_and_archive_retention_are_required() {
    assert_eq!(
        connection("", 0, "bucket", "/folder", 0, ""),
        Err("Connection name is required.".to_owned())
    );
    assert_eq!(
        connection("Photos", 0, "bucket", "/folder", 2, "0"),
        Err("Enter a whole number of at least 1 for archive retention.".to_owned())
    );
}

#[test]
fn provider_name_must_not_exceed_the_shared_limit() {
    assert_eq!(valid_provider(&"a".repeat(NAME_MAX_LENGTH)), Ok(()));
    assert_eq!(
        valid_provider(&"a".repeat(NAME_MAX_LENGTH + 1)),
        Err("Provider name must not exceed 60 characters".to_owned())
    );
}

#[test]
fn connection_name_must_not_exceed_the_shared_limit() {
    assert_eq!(
        connection(&"a".repeat(NAME_MAX_LENGTH), 0, "bucket", "/folder", 0, ""),
        Ok(())
    );
    assert_eq!(
        connection(
            &"a".repeat(NAME_MAX_LENGTH + 1),
            0,
            "bucket",
            "/folder",
            0,
            ""
        ),
        Err("Connection name must not exceed 60 characters".to_owned())
    );
}

#[test]
fn credential_errors_match_each_providers_form_labels_and_focus() {
    for (kind, access_error, secret_error) in [
        (
            ProviderKind::BackblazeB2,
            "Key ID is required.",
            "Application key is required.",
        ),
        (
            ProviderKind::AwsS3,
            "Access key ID is required.",
            "Secret access key is required.",
        ),
        (
            ProviderKind::CloudflareR2,
            "Access key ID is required.",
            "Secret access key is required.",
        ),
    ] {
        for (access, secret, expected) in [("", "", access_error), ("access", "  ", secret_error)] {
            let error = provider(ProviderFields {
                name: "Cloud",
                access_key_id: access,
                secret_access_key: secret,
                kind,
                account_id: "account",
                region: "",
                default_bucket: "",
                endpoint: "",
            })
            .unwrap_err();
            assert_eq!(error, expected);
            assert_eq!(super::provider_focus(&error), 2);
        }
    }
}
