use crate::{
    AppWindow,
    configuration::{ConfigStore, ProviderId, ProviderKind, ProviderOptions},
};
use slint::{ModelRc, SharedString, VecModel};

/// Removes credentials and verification data that only belong to the current form session.
pub(crate) fn clear_transient_state(window: &AppWindow) {
    window.set_provider_form_access_key(SharedString::default());
    window.set_provider_form_secret_key(SharedString::default());
    window.set_provider_form_session_token(SharedString::default());
    crate::provider_secret_reveal_controller::hide(window);
    window.set_provider_advanced_expanded(false);
    invalidate_verification(window);
    window.set_provider_verified_buckets(ModelRc::new(std::rc::Rc::new(VecModel::default())));
    window.set_provider_bucket_list_empty(false);
}

/// Starts a distinct verification attempt for the currently visible provider form.
pub(crate) fn begin_verification(window: &AppWindow) -> i32 {
    let generation = next_verification_generation(window.get_provider_verification_generation());
    window.set_provider_verification_generation(generation);
    window.set_provider_verifying(true);
    generation
}

/// Prevents a late verification worker from updating a form that has been left or reset.
pub(crate) fn invalidate_verification(window: &AppWindow) {
    window.set_provider_verification_generation(next_verification_generation(
        window.get_provider_verification_generation(),
    ));
    window.set_provider_verifying(false);
    window.set_provider_save_after_verification(false);
}

#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
pub(crate) fn is_current_verification(expected: i32, current: i32) -> bool {
    expected == current
}

fn next_verification_generation(current: i32) -> i32 {
    current.wrapping_add(1)
}

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

#[cfg(test)]
mod tests {
    use super::{is_current_verification, next_verification_generation};

    #[test]
    fn only_the_current_verification_attempt_can_update_a_form() {
        assert!(is_current_verification(8, 8));
        assert!(!is_current_verification(8, 9));
    }

    #[test]
    fn verification_generation_advances_across_integer_wraparound() {
        assert_eq!(next_verification_generation(8), 9);
        assert_eq!(next_verification_generation(i32::MAX), i32::MIN);
    }
}
