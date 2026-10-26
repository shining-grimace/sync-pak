use crate::{configuration::ProviderKind, validation::validate_name_length};

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
    validate_name_length(name, "Provider name")?;
    if kind == ProviderKind::CloudflareR2 && endpoint.trim().is_empty() {
        required(account_id, "Account ID")?;
    }
    let (access_label, secret_label) = match kind {
        ProviderKind::BackblazeB2 => ("Key ID", "Application key"),
        _ => ("Access key ID", "Secret access key"),
    };
    required(access_key_id, access_label)?;
    required(secret_access_key, secret_label)?;
    // Connection defaults appear above the fixed bucket in the form.
    let defaults = super::transport_form::fields(kind, endpoint, region);
    if defaults.endpoint.is_some() || defaults.region.is_some() {
        defaults
            .resolve(kind, Some(account_id))
            .map_err(str::to_owned)?;
    }
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
    validate_name_length(name, "Connection name")?;
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

pub(crate) fn directions(allow_upload: bool, allow_download: bool) -> Result<(), String> {
    (allow_upload || allow_download)
        .then_some(())
        .ok_or_else(|| "Choose at least one allowed direction.".to_owned())
}

pub(crate) fn provider_focus(error: &str) -> i32 {
    if error.starts_with("Provider name") {
        1
    } else if error.starts_with("Access key")
        || error.starts_with("Secret access")
        || error.starts_with("Key ID")
        || error.starts_with("Application key")
    {
        2
    } else {
        1
    }
}

pub(crate) fn connection_focus(error: &str) -> i32 {
    if error.starts_with("Connection name") {
        1
    } else if error.starts_with("Choose a provider") {
        2
    } else if error.starts_with("Bucket") {
        3
    } else if error.starts_with("Local folder") {
        4
    } else if error.starts_with("Enter a whole number") {
        5
    } else {
        1
    }
}

fn required(value: &str, label: &str) -> Result<(), String> {
    (!value.trim().is_empty())
        .then_some(())
        .ok_or_else(|| format!("{label} is required."))
}

#[cfg(test)]
mod tests;
