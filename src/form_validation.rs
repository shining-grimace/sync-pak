pub(crate) fn provider(
    name: &str,
    access_key_id: &str,
    secret_access_key: &str,
) -> Result<(), String> {
    required(name, "Provider name")?;
    required(access_key_id, "Access key ID")?;
    required(secret_access_key, "Secret access key")
}

fn required(value: &str, label: &str) -> Result<(), String> {
    (!value.trim().is_empty())
        .then_some(())
        .ok_or_else(|| format!("{label} is required."))
}

#[cfg(test)]
mod tests {
    use super::provider;

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
}
