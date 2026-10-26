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
            provider(
                name,
                access,
                secret,
                ProviderKind::CloudflareR2,
                account,
                "",
                bucket,
                ""
            ),
            Err(expected.into()),
        );
    }
}

#[test]
fn r2_with_an_explicit_endpoint_does_not_require_an_account_id() {
    assert_eq!(
        provider(
            "Cloud",
            "",
            "",
            ProviderKind::CloudflareR2,
            "",
            "",
            "",
            "https://example.com"
        ),
        Err("Access key ID is required.".into()),
    );
    assert_eq!(
        provider(
            "Cloud",
            "access",
            "secret",
            ProviderKind::CloudflareR2,
            "",
            "",
            "",
            "https://example.com"
        ),
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
            provider(
                "Cloud",
                "access",
                "secret",
                kind,
                "account",
                "",
                " bucket",
                "ftp://example.com"
            ),
            Err("S3 API endpoint must use HTTPS or HTTP.".into()),
        );
    }
}
