use crate::configuration::ProviderKind;

pub(crate) fn provider(
    name: &str,
    access_key_id: &str,
    secret_access_key: &str,
    kind: ProviderKind,
    account_id: &str,
    region: &str,
    default_bucket: &str,
    endpoint: &str,
) -> Result<(), String> {
    required(name, "Provider name")?;
    required(access_key_id, "Access key ID")?;
    required(secret_access_key, "Secret access key")?;
    match kind {
        ProviderKind::CloudflareR2 => required(account_id, "Account ID"),
        ProviderKind::BackblazeB2 => {
            required(region, "Region")?;
            required(endpoint, "Custom endpoint")
        }
        ProviderKind::AwsS3 => required(region, "Region"),
    }?;
    validate_optional_bucket(default_bucket)
}

fn validate_optional_bucket(bucket: &str) -> Result<(), String> {
    if !bucket.trim().is_empty() && bucket != bucket.trim() {
        return Err("Bucket name cannot begin or end with whitespace.".to_owned());
    }
    Ok(())
}

pub(crate) fn connection(
    name: &str,
    provider_index: i32,
    bucket: &str,
    local_path: &str,
    mode_index: i32,
    retention: &str,
) -> Result<(), String> {
    required(name, "Connection name")?;
    (provider_index >= 0)
        .then_some(())
        .ok_or_else(|| "Choose a provider.".to_owned())?;
    required(bucket, "Bucket")?;
    required(local_path, "Local folder")?;
    if mode_index == 2 {
        retention
            .trim()
            .parse::<u32>()
            .ok()
            .filter(|count| *count >= 1)
            .map(|_| ())
            .ok_or_else(|| {
                "Enter a whole number of at least 1 for archive retention.".to_owned()
            })?;
    }
    Ok(())
}

fn required(value: &str, label: &str) -> Result<(), String> {
    (!value.trim().is_empty())
        .then_some(())
        .ok_or_else(|| format!("{label} is required."))
}

#[cfg(test)]
mod tests {
    use super::{connection, provider};
    use crate::configuration::ProviderKind;

    #[test]
    fn provider_credentials_are_required() {
        assert_eq!(
            provider(
                "Provider",
                "",
                "secret",
                ProviderKind::AwsS3,
                "",
                "region",
                "",
                ""
            ),
            Err("Access key ID is required.".to_owned())
        );
        assert_eq!(
            provider(
                "Provider",
                "access",
                "  ",
                ProviderKind::AwsS3,
                "",
                "region",
                "",
                ""
            ),
            Err("Secret access key is required.".to_owned())
        );
    }

    #[test]
    fn provider_specific_metadata_is_required() {
        assert_eq!(
            provider(
                "Provider",
                "access",
                "secret",
                ProviderKind::CloudflareR2,
                "",
                "",
                "",
                ""
            ),
            Err("Account ID is required.".to_owned())
        );
        assert_eq!(
            provider(
                "Provider",
                "access",
                "secret",
                ProviderKind::AwsS3,
                "",
                "",
                "",
                ""
            ),
            Err("Region is required.".to_owned())
        );
        assert_eq!(
            provider(
                "Provider",
                "access",
                "secret",
                ProviderKind::BackblazeB2,
                "",
                "us-west-004",
                "",
                ""
            ),
            Err("Custom endpoint is required.".to_owned())
        );
    }

    #[test]
    fn optional_bucket_cannot_have_outer_whitespace() {
        assert_eq!(
            provider(
                "Provider",
                "access",
                "secret",
                ProviderKind::AwsS3,
                "",
                "region",
                " bucket",
                ""
            ),
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
}
