use crate::{
    AppWindow,
    configuration::{ConfigStore, ProviderId, ProviderKind, ProviderOptions},
};

/// Stores a non-secret form fingerprint for unsaved-change detection.
pub(crate) fn mark_clean(window: &AppWindow) {
    window.set_provider_form_original(form_signature(window).into());
}

pub(crate) fn is_dirty(window: &AppWindow) -> bool {
    window.get_provider_form_original() != form_signature(window).as_str()
}

fn form_signature(window: &AppWindow) -> String {
    format!(
        "{:?}",
        (
            window.get_provider_form_name(),
            window.get_provider_form_kind(),
            window.get_provider_form_account_id(),
            window.get_provider_form_region(),
            window.get_provider_form_bucket(),
            window.get_provider_form_endpoint(),
            !window.get_provider_form_access_key().is_empty(),
            !window.get_provider_form_secret_key().is_empty(),
            !window.get_provider_form_session_token().is_empty(),
        )
    )
}

pub(crate) fn provider_kind(index: i32) -> Option<ProviderKind> {
    match index {
        0 => Some(ProviderKind::CloudflareR2),
        1 => Some(ProviderKind::BackblazeB2),
        2 => Some(ProviderKind::AwsS3),
        _ => None,
    }
}

pub(crate) fn provider_kind_index(kind: ProviderKind) -> i32 {
    match kind {
        ProviderKind::CloudflareR2 => 0,
        ProviderKind::BackblazeB2 => 1,
        ProviderKind::AwsS3 => 2,
    }
}

pub(crate) fn provider_options(
    account_id: &str,
    region: &str,
    default_bucket: &str,
    endpoint: &str,
) -> ProviderOptions {
    ProviderOptions {
        account_id: optional(account_id),
        default_bucket: optional(default_bucket),
        endpoint: optional(endpoint),
        region: optional(region),
    }
}

fn optional(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_owned())
}

pub(crate) fn provider_id(configuration: &ConfigStore, id: &str) -> Result<ProviderId, String> {
    configuration
        .load()
        .map_err(|error| error.to_string())?
        .providers
        .into_iter()
        .find(|provider| provider.id.as_str() == id)
        .map(|provider| provider.id)
        .ok_or_else(|| "The provider no longer exists.".to_owned())
}
