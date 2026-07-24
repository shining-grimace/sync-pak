use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};

use crate::{
    AppWindow,
    configuration::{
        ConfigStore, ConnectionConfig, ConnectionDraft, ProviderConfig, ProviderId, SyncMode,
    },
};

pub(crate) fn reset(window: &AppWindow) {
    window.set_connection_form_id(SharedString::default());
    window.set_connection_form_name(SharedString::default());
    window.set_connection_form_provider(0);
    window.set_connection_form_bucket(SharedString::default());
    window.set_connection_form_remote(SharedString::default());
    window.set_connection_form_local(SharedString::default());
    window.set_connection_form_mode(0);
    window.set_connection_form_retention("1".into());
    set_verified_buckets(window, None);
    mark_clean(window);
}

/// Removes the current connection draft and any session-only bucket listing.
pub(crate) fn clear_transient_state(window: &AppWindow) {
    reset(window);
    window.set_provider_names(ModelRc::new(Rc::new(VecModel::default())));
    window.set_provider_buckets(ModelRc::new(Rc::new(VecModel::default())));
}

pub(crate) fn populate(
    window: &AppWindow,
    providers: &[ProviderConfig],
    connection: ConnectionConfig,
    provider_index: usize,
) {
    set_provider_models(window, providers);
    window.set_connection_form_id(connection.id.as_str().into());
    window.set_connection_form_name(connection.name.into());
    window.set_connection_form_provider(provider_index as i32);
    window.set_connection_form_bucket(connection.bucket.into());
    window.set_connection_form_remote(connection.remote_path.into());
    window.set_connection_form_local(connection.local_path.into());
    window.set_connection_form_mode(mode_index(connection.mode));
    window.set_connection_form_retention(
        connection
            .keep_last_archives
            .unwrap_or(1)
            .to_string()
            .into(),
    );
    set_verified_buckets(window, None);
    mark_clean(window);
    window.set_status_message(SharedString::default());
    window.set_page(5);
}

pub(crate) fn mark_clean(window: &AppWindow) {
    window.set_connection_form_original(form_signature(window).into());
}

pub(crate) fn is_dirty(window: &AppWindow) -> bool {
    window.get_connection_form_original() != form_signature(window).as_str()
}

fn form_signature(window: &AppWindow) -> String {
    format!(
        "{:?}",
        (
            window.get_connection_form_name(),
            window.get_connection_form_provider(),
            window.get_connection_form_bucket(),
            window.get_connection_form_remote(),
            window.get_connection_form_local(),
            window.get_connection_form_mode(),
            window.get_connection_form_retention(),
        )
    )
}

pub(crate) fn set_provider_models(window: &AppWindow, providers: &[ProviderConfig]) {
    window.set_provider_names(ModelRc::new(Rc::new(VecModel::from_iter(
        providers
            .iter()
            .map(|provider| SharedString::from(&provider.name)),
    ))));
    window.set_provider_buckets(ModelRc::new(Rc::new(VecModel::from_iter(
        providers.iter().map(|provider| {
            SharedString::from(
                provider
                    .options
                    .default_bucket
                    .as_deref()
                    .unwrap_or_default(),
            )
        }),
    ))));
}

pub(crate) fn set_provider_bucket(window: &AppWindow, providers: &[ProviderConfig], index: i32) {
    if let Some(bucket) = provider_bucket(providers, index) {
        window.set_connection_form_bucket(bucket.into());
    }
}

pub(crate) fn set_verified_buckets(window: &AppWindow, buckets: Option<Vec<String>>) {
    let listed = buckets.is_some();
    let buckets = buckets.unwrap_or_default();
    window.set_connection_bucket_list_checked(listed);
    window.set_connection_verified_buckets(ModelRc::new(Rc::new(VecModel::from_iter(
        buckets.into_iter().map(SharedString::from),
    ))));
}

fn provider_bucket(providers: &[ProviderConfig], index: i32) -> Option<&str> {
    usize::try_from(index)
        .ok()
        .and_then(|index| providers.get(index))
        .and_then(|provider| provider.options.default_bucket.as_deref())
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn draft(
    configuration: &ConfigStore,
    name: SharedString,
    provider_index: i32,
    bucket: SharedString,
    remote_path: SharedString,
    local_path: SharedString,
    mode_index: i32,
    retention: SharedString,
) -> Result<ConnectionDraft, String> {
    let config = configuration.load().map_err(|error| error.to_string())?;
    let provider_id = provider_id(&config.providers, provider_index)?;
    let mode = mode(mode_index)?;
    let keep_last_archives = archive_retention(mode, &retention)?;
    Ok(ConnectionDraft {
        name: name.to_string(),
        provider_id,
        bucket: bucket.to_string(),
        remote_path: remote_path.to_string(),
        local_path: local_path.to_string(),
        mode,
        keep_last_archives,
    })
}

pub(crate) fn existing_id(
    configuration: &ConfigStore,
    id: &str,
) -> Result<crate::configuration::ConnectionId, String> {
    configuration
        .load()
        .map_err(|error| error.to_string())?
        .connections
        .into_iter()
        .find(|connection| connection.id.as_str() == id)
        .map(|connection| connection.id)
        .ok_or_else(|| "The connection no longer exists.".to_owned())
}

fn provider_id(providers: &[ProviderConfig], index: i32) -> Result<ProviderId, String> {
    usize::try_from(index)
        .ok()
        .and_then(|index| providers.get(index))
        .map(|provider| provider.id.clone())
        .ok_or_else(|| "Choose a provider.".to_owned())
}

fn mode(index: i32) -> Result<SyncMode, String> {
    match index {
        0 => Ok(SyncMode::AddOnly),
        1 => Ok(SyncMode::Mirror),
        2 => Ok(SyncMode::Archive),
        _ => Err("Choose a mode.".to_owned()),
    }
}

fn mode_index(mode: SyncMode) -> i32 {
    match mode {
        SyncMode::AddOnly => 0,
        SyncMode::Mirror => 1,
        SyncMode::Archive => 2,
    }
}

fn archive_retention(mode: SyncMode, input: &str) -> Result<Option<u32>, String> {
    if !matches!(mode, SyncMode::Archive) {
        return Ok(None);
    }
    input
        .trim()
        .parse::<u32>()
        .ok()
        .filter(|count| *count >= 1)
        .map(Some)
        .ok_or_else(|| "Enter a whole number of at least 1 for archive retention.".to_owned())
}

#[cfg(test)]
#[path = "connection_form_data_tests.rs"]
mod tests;
