use std::collections::HashSet;

use super::{AppConfig, CURRENT_SCHEMA_VERSION, ProviderKind, SyncMode};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ValidationErrors(Vec<String>);

impl ValidationErrors {
    pub fn messages(&self) -> &[String] {
        &self.0
    }
}

impl std::fmt::Display for ValidationErrors {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.0.join(" "))
    }
}

impl std::error::Error for ValidationErrors {}

pub(super) fn validate(config: &AppConfig) -> Result<(), ValidationErrors> {
    let mut errors = Vec::new();
    if config.schema_version != CURRENT_SCHEMA_VERSION {
        errors.push("The configuration schema version is not current.".to_owned());
    }
    unique_provider_ids(config, &mut errors);
    unique_connection_ids(config, &mut errors);
    for provider in &config.providers {
        required(&provider.name, "Provider name", &mut errors);
        validate_provider_options(provider, &mut errors);
        if provider.id != provider.credential_reference.provider_id {
            errors.push("A provider credential reference must use its provider ID.".to_owned());
        }
    }
    for connection in &config.connections {
        required(&connection.name, "Connection name", &mut errors);
        required(&connection.bucket, "Bucket", &mut errors);
        required(&connection.local_path, "Local folder", &mut errors);
        if !config
            .providers
            .iter()
            .any(|provider| provider.id == connection.provider_id)
        {
            errors.push(format!(
                "Connection '{}' refers to a missing provider.",
                connection.name
            ));
        }
        if matches!(connection.mode, SyncMode::Archive)
            && connection.keep_last_archives.unwrap_or(0) < 1
        {
            errors.push("Archive connections must keep at least one archive.".to_owned());
        }
        if !matches!(connection.mode, SyncMode::Archive) && connection.keep_last_archives.is_some()
        {
            errors.push("Only archive connections can set archive retention.".to_owned());
        }
    }
    (!errors.is_empty())
        .then_some(ValidationErrors(errors))
        .map_or(Ok(()), Err)
}

fn required(value: &str, label: &str, errors: &mut Vec<String>) {
    if value.trim().is_empty() {
        errors.push(format!("{label} is required."));
    }
}

fn validate_provider_options(provider: &super::ProviderConfig, errors: &mut Vec<String>) {
    match provider.kind {
        ProviderKind::CloudflareR2 => required(
            provider.options.account_id.as_deref().unwrap_or_default(),
            "Cloudflare R2 account ID",
            errors,
        ),
        ProviderKind::BackblazeB2 | ProviderKind::AwsS3 => required(
            provider.options.region.as_deref().unwrap_or_default(),
            "Provider region",
            errors,
        ),
    }
}

fn unique_provider_ids(config: &AppConfig, errors: &mut Vec<String>) {
    let ids: HashSet<_> = config
        .providers
        .iter()
        .map(|provider| provider.id.as_str())
        .collect();
    if ids.len() != config.providers.len() {
        errors.push("Provider IDs must be unique.".to_owned());
    }
}

fn unique_connection_ids(config: &AppConfig, errors: &mut Vec<String>) {
    let ids: HashSet<_> = config
        .connections
        .iter()
        .map(|connection| connection.id.as_str())
        .collect();
    if ids.len() != config.connections.len() {
        errors.push("Connection IDs must be unique.".to_owned());
    }
}
