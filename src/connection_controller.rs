use std::{rc::Rc, time::Duration};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{
    AppWindow, ConnectionRow,
    configuration::{ConfigStore, ConnectionDraft, ConnectionRepository, ProviderId, SyncMode},
};

pub(crate) fn configure(window: &AppWindow, configuration: &Rc<ConfigStore>) {
    let weak = window.as_weak();
    let connections_config = Rc::clone(configuration);
    window.on_show_connections(move || show_connections(&weak, Rc::clone(&connections_config)));

    let weak = window.as_weak();
    let form_config = Rc::clone(configuration);
    window.on_show_add_connection(move || show_add_connection(&weak, Rc::clone(&form_config)));

    let weak = window.as_weak();
    let save_config = Rc::clone(configuration);
    window.on_save_connection(
        move |name, provider, bucket, remote, local, mode, retention| {
            save_connection(
                &weak,
                Rc::clone(&save_config),
                name,
                provider,
                bucket,
                remote,
                local,
                mode,
                retention,
            );
        },
    );
}

fn show_connections(weak: &slint::Weak<AppWindow>, configuration: Rc<ConfigStore>) {
    let Some(window) = weak.upgrade() else { return };
    window.set_status_message(SharedString::default());
    window.set_page(4);
    let weak = weak.clone();
    slint::Timer::single_shot(Duration::ZERO, move || {
        refresh_connections(&weak, &configuration)
    });
}

fn refresh_connections(weak: &slint::Weak<AppWindow>, configuration: &ConfigStore) {
    let Some(window) = weak.upgrade() else { return };
    match configuration.load() {
        Ok(config) => {
            let rows = config.connections.iter().map(|connection| {
                let provider = config
                    .providers
                    .iter()
                    .find(|provider| provider.id == connection.provider_id)
                    .map(|provider| provider.name.as_str())
                    .unwrap_or("Unavailable provider");
                ConnectionRow {
                    name: connection.name.clone().into(),
                    detail: format!(
                        "On this device → In {provider} · {}",
                        mode_name(connection.mode)
                    )
                    .into(),
                }
            });
            window.set_connections(ModelRc::new(Rc::new(VecModel::from_iter(rows))));
            window.set_status_message(SharedString::default());
        }
        Err(error) => {
            window.set_status_message(format!("SyncPak could not load connections: {error}").into())
        }
    }
}

fn show_add_connection(weak: &slint::Weak<AppWindow>, configuration: Rc<ConfigStore>) {
    let Some(window) = weak.upgrade() else { return };
    window.set_status_message(SharedString::default());
    window.set_page(5);
    let weak = weak.clone();
    slint::Timer::single_shot(Duration::ZERO, move || {
        load_provider_names(&weak, &configuration)
    });
}

fn load_provider_names(weak: &slint::Weak<AppWindow>, configuration: &ConfigStore) {
    let Some(window) = weak.upgrade() else { return };
    match configuration.load() {
        Ok(config) => window.set_provider_names(ModelRc::new(Rc::new(VecModel::from_iter(
            config
                .providers
                .into_iter()
                .map(|provider| SharedString::from(provider.name)),
        )))),
        Err(error) => {
            window.set_status_message(format!("SyncPak could not load providers: {error}").into())
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn save_connection(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    name: SharedString,
    provider_index: i32,
    bucket: SharedString,
    remote_path: SharedString,
    local_path: SharedString,
    mode_index: i32,
    retention: SharedString,
) {
    let Some(window) = weak.upgrade() else { return };
    let result = draft_from_input(
        &configuration,
        name,
        provider_index,
        bucket,
        remote_path,
        local_path,
        mode_index,
        retention,
    )
    .and_then(|draft| {
        ConnectionRepository::new(&configuration)
            .create(draft)
            .map_err(|error| error.to_string())
    });
    match result {
        Ok(_) => show_connections(weak, configuration),
        Err(error) => window.set_status_message(error.into()),
    }
}

#[allow(clippy::too_many_arguments)]
fn draft_from_input(
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
    let mode = sync_mode(mode_index)?;
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

fn provider_id(
    providers: &[crate::configuration::ProviderConfig],
    index: i32,
) -> Result<ProviderId, String> {
    usize::try_from(index)
        .ok()
        .and_then(|index| providers.get(index))
        .map(|provider| provider.id.clone())
        .ok_or_else(|| "Choose a provider.".to_owned())
}

fn sync_mode(index: i32) -> Result<SyncMode, String> {
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

fn mode_name(mode: SyncMode) -> &'static str {
    match mode {
        SyncMode::AddOnly => "Add-only",
        SyncMode::Mirror => "Mirror",
        SyncMode::Archive => "Archive",
    }
}
