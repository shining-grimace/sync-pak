use crate::{
    AppWindow,
    app::form_validation,
    configuration::{
        ConfigStore, ConnectionDraft, ProviderConfig, ProviderId, SyncMode, TransportSettings,
    },
};
use slint::SharedString;

#[allow(clippy::too_many_arguments)]
pub(crate) fn draft(
    configuration: &ConfigStore,
    transport: Option<TransportSettings>,
    name: SharedString,
    provider_index: i32,
    bucket: SharedString,
    remote_path: SharedString,
    local_path: SharedString,
    mode_index: i32,
    retention: SharedString,
    allow_upload: bool,
    allow_download: bool,
    show_preview: bool,
) -> Result<ConnectionDraft, String> {
    let config = configuration.load().map_err(|error| error.to_string())?;
    let provider_id = provider_id(&config.providers, provider_index)?;
    let mode = mode(mode_index)?;
    crate::configuration::validate_run_policy(mode, allow_upload, allow_download, show_preview)?;
    let keep_last_archives = archive_retention(mode, &retention)?;
    Ok(ConnectionDraft {
        transport,
        name: name.to_string(),
        provider_id,
        bucket: bucket.to_string(),
        remote_path: remote_path.to_string(),
        local_path: local_path.to_string(),
        mode,
        allow_upload,
        allow_download,
        show_preview,
        keep_last_archives,
        verified: false,
    })
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

pub(super) fn validated_form_draft(
    window: &AppWindow,
    configuration: &ConfigStore,
) -> Option<ConnectionDraft> {
    let name = window.get_connection_form_name();
    let provider = window.get_connection_form_provider();
    let bucket = window.get_connection_form_bucket();
    let remote = window.get_connection_form_remote();
    let local = window.get_connection_form_local();
    let mode = window.get_connection_form_mode();
    let retention = window.get_connection_form_retention();
    if let Err(error) = form_validation::directions(
        window.get_connection_form_allow_upload(),
        window.get_connection_form_allow_download(),
    ) {
        window.set_status_message(error.into());
        return None;
    }
    if let Err(error) =
        form_validation::connection(&name, provider, &bucket, &local, mode, &retention)
    {
        window.set_connection_validation_focus(form_validation::connection_focus(&error));
        window.set_status_message(error.into());
        return None;
    }
    let transport = match super::transport::read(window, configuration) {
        Ok(settings) => settings,
        Err(error) => {
            window.set_status_message(error.into());
            return None;
        }
    };
    match draft(
        configuration,
        transport,
        name,
        provider,
        bucket,
        remote,
        local,
        mode,
        retention,
        window.get_connection_form_allow_upload(),
        window.get_connection_form_allow_download(),
        window.get_connection_form_show_preview(),
    ) {
        Ok(connection) => Some(connection),
        Err(error) => {
            window.set_status_message(error.into());
            None
        }
    }
}
