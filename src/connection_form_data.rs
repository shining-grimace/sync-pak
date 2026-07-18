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
    set_provider_bucket(window, providers, provider_index as i32);
    window.set_status_message(SharedString::default());
    window.set_page(5);
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
    if let Some(provider) = usize::try_from(index)
        .ok()
        .and_then(|index| providers.get(index))
    {
        window.set_connection_form_bucket(
            provider
                .options
                .default_bucket
                .as_deref()
                .unwrap_or_default()
                .into(),
        );
    }
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
