pub(crate) fn provider(
    name: &str,
    access_key_id: &str,
    secret_access_key: &str,
) -> Result<(), String> {
    required(name, "Provider name")?;
    required(access_key_id, "Access key ID")?;
    required(secret_access_key, "Secret access key")
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

    #[test]
    fn provider_credentials_are_required() {
        assert_eq!(
            provider("Provider", "", "secret"),
            Err("Access key ID is required.".to_owned())
        );
        assert_eq!(
            provider("Provider", "access", "  "),
            Err("Secret access key is required.".to_owned())
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
