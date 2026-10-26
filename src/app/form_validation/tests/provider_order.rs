use super::*;

#[test]
fn r2_reports_missing_fields_in_visual_order() {
    for (name, account, access, secret, bucket, expected) in [
        ("", "", "", "", " bucket", "Provider name is required."),
        ("Cloud", "", "", "", " bucket", "Account ID is required."),
        (
            "Cloud",
            "account",
            "",
            "",
            " bucket",
            "Access key ID is required.",
        ),
        (
            "Cloud",
            "account",
            "access",
            "",
            " bucket",
            "Secret access key is required.",
        ),
        (
            "Cloud",
            "account",
            "access",
            "secret",
            " bucket",
            "Bucket name cannot begin or end with whitespace.",
        ),
    ] {
        assert_eq!(
            provider(ProviderFields {
                name,
                access_key_id: access,
                secret_access_key: secret,
                kind: ProviderKind::CloudflareR2,
                account_id: account,
                region: "",
                default_bucket: bucket,
                endpoint: "",
            }),
            Err(expected.into()),
        );
    }
}

#[test]
fn r2_with_an_explicit_endpoint_does_not_require_an_account_id() {
    assert_eq!(
        provider(ProviderFields {
            name: "Cloud",
            access_key_id: "",
            secret_access_key: "",
            kind: ProviderKind::CloudflareR2,
            account_id: "",
            region: "",
            default_bucket: "",
            endpoint: "https://example.com",
        }),
        Err("Access key ID is required.".into()),
    );
    assert_eq!(
        provider(ProviderFields {
            name: "Cloud",
            access_key_id: "access",
            secret_access_key: "secret",
            kind: ProviderKind::CloudflareR2,
            account_id: "",
            region: "",
            default_bucket: "",
            endpoint: "https://example.com",
        }),
        Ok(()),
    );
}

#[test]
fn provider_defaults_are_validated_before_the_fixed_bucket() {
    for kind in [
        ProviderKind::CloudflareR2,
        ProviderKind::BackblazeB2,
        ProviderKind::AwsS3,
    ] {
        assert_eq!(
            provider(ProviderFields {
                name: "Cloud",
                access_key_id: "access",
                secret_access_key: "secret",
                kind,
                account_id: "account",
                region: "",
                default_bucket: " bucket",
                endpoint: "ftp://example.com",
            }),
            Err("S3 API endpoint must use HTTPS or HTTP.".into()),
        );
    }
}
