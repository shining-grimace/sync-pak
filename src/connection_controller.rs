use std::{rc::Rc, time::Duration};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{
    AppWindow, ConnectionRow,
    configuration::{ConfigStore, ConnectionDraft, ConnectionRepository, ProviderId, SyncMode},
    diagnostics_controller::{self, SharedDiagnosticLog},
    form_validation,
};

pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let weak = window.as_weak();
    let connections_config = Rc::clone(configuration);
    let connections_diagnostics = Rc::clone(&diagnostics);
    window.on_show_connections(move || {
        show_connections(
            &weak,
            Rc::clone(&connections_config),
            Rc::clone(&connections_diagnostics),
        )
    });

    let weak = window.as_weak();
    let form_config = Rc::clone(configuration);
    let form_diagnostics = Rc::clone(&diagnostics);
    window.on_show_add_connection(move || {
        show_add_connection(&weak, Rc::clone(&form_config), Rc::clone(&form_diagnostics))
    });

    let weak = window.as_weak();
    let save_config = Rc::clone(configuration);
    let save_diagnostics = Rc::clone(&diagnostics);
    window.on_save_connection(
        move |name, provider, bucket, remote, local, mode, retention| {
            save_connection(
                &weak,
                Rc::clone(&save_config),
                Rc::clone(&save_diagnostics),
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

    let weak = window.as_weak();
    let edit_config = Rc::clone(configuration);
    let edit_diagnostics = Rc::clone(&diagnostics);
    window.on_request_connection_edit(move |id| {
        request_edit(&weak, &edit_config, &edit_diagnostics, id)
    });

    let weak = window.as_weak();
    let bucket_config = Rc::clone(configuration);
    let bucket_diagnostics = Rc::clone(&diagnostics);
    window.on_select_connection_provider(move |index| {
        select_provider_bucket(&weak, &bucket_config, &bucket_diagnostics, index);
    });
}

pub(crate) fn show_connections(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    window.set_status_message(SharedString::default());
    window.set_page(4);
    let weak = weak.clone();
    slint::Timer::single_shot(Duration::ZERO, move || {
        refresh_connections(&weak, &configuration, &diagnostics)
    });
}

fn refresh_connections(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
) {
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
                    id: connection.id.as_str().into(),
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
        Err(_) => diagnostics_controller::present(
            &window,
            diagnostics,
            "Connections could not be loaded",
            "connection configuration load failed",
            "SyncPak could not load connections. Check configuration storage and try again.",
        ),
    }
}

fn show_add_connection(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    window.set_status_message(SharedString::default());
    reset_form(&window);
    window.set_page(5);
    let weak = weak.clone();
    slint::Timer::single_shot(Duration::ZERO, move || {
        load_provider_names(&weak, &configuration, &diagnostics)
    });
}

fn load_provider_names(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    match configuration.load() {
        Ok(config) => {
            set_provider_models(&window, &config.providers);
            set_provider_bucket(
                &window,
                &config.providers,
                window.get_connection_form_provider(),
            );
        }
        Err(_) => diagnostics_controller::present(
            &window,
            diagnostics,
            "Providers could not be loaded for a connection",
            "provider configuration load failed",
            "SyncPak could not load providers. Check configuration storage and try again.",
        ),
    }
}

#[allow(clippy::too_many_arguments)]
fn save_connection(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    name: SharedString,
    provider_index: i32,
    bucket: SharedString,
    remote_path: SharedString,
    local_path: SharedString,
    mode_index: i32,
    retention: SharedString,
) {
    let Some(window) = weak.upgrade() else { return };
    if let Err(error) = form_validation::connection(
        &name,
        provider_index,
        &bucket,
        &local_path,
        mode_index,
        &retention,
    ) {
        window.set_status_message(error.into());
        return;
    }
    let edit_id = window.get_connection_form_id();
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
        let repository = ConnectionRepository::new(&configuration);
        if edit_id.is_empty() {
            repository.create(draft).map_err(|error| error.to_string())
        } else {
            connection_id(&configuration, edit_id.as_str()).and_then(|id| {
                repository
                    .update(&id, draft)
                    .map_err(|error| error.to_string())
            })
        }
    });
    match result {
        Ok(_) => show_connections(weak, configuration, diagnostics),
        Err(_) => diagnostics_controller::present(
            &window,
            &diagnostics,
            "Connection could not be saved",
            "connection save failed",
            "SyncPak could not save this connection. Check configuration storage and try again.",
        ),
    }
}

fn request_edit(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
    id: SharedString,
) {
    let Some(window) = weak.upgrade() else { return };
    let result = configuration
        .load()
        .map_err(|error| error.to_string())
        .and_then(|config| {
            let connection = config
                .connections
                .iter()
                .find(|connection| id == connection.id.as_str())
                .cloned()
                .ok_or_else(|| "The connection no longer exists.".to_owned())?;
            let provider_index = config
                .providers
                .iter()
                .position(|provider| provider.id == connection.provider_id)
                .ok_or_else(|| "The connection's provider no longer exists.".to_owned())?;
            Ok((config.providers, connection, provider_index))
        });
    match result {
        Ok((providers, connection, provider_index)) => {
            set_provider_models(&window, &providers);
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
            set_provider_bucket(&window, &providers, provider_index as i32);
            window.set_status_message(SharedString::default());
            window.set_page(5);
        }
        Err(_) => diagnostics_controller::present(
            &window,
            diagnostics,
            "Connection could not be opened",
            "connection edit load failed",
            "SyncPak could not open this connection. It may have been removed.",
        ),
    }
}

fn select_provider_bucket(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
    index: i32,
) {
    let Some(window) = weak.upgrade() else { return };
    match configuration.load() {
        Ok(config) => set_provider_bucket(&window, &config.providers, index),
        Err(_) => diagnostics_controller::present(
            &window,
            diagnostics,
            "Providers could not be loaded for a connection",
            "provider configuration load failed",
            "SyncPak could not load providers. Check configuration storage and try again.",
        ),
    }
}

fn set_provider_models(window: &AppWindow, providers: &[crate::configuration::ProviderConfig]) {
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

fn set_provider_bucket(
    window: &AppWindow,
    providers: &[crate::configuration::ProviderConfig],
    index: i32,
) {
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

fn reset_form(window: &AppWindow) {
    window.set_connection_form_id(SharedString::default());
    window.set_connection_form_name(SharedString::default());
    window.set_connection_form_provider(0);
    window.set_connection_form_bucket(SharedString::default());
    window.set_connection_form_remote(SharedString::default());
    window.set_connection_form_local(SharedString::default());
    window.set_connection_form_mode(0);
    window.set_connection_form_retention("1".into());
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

fn mode_index(mode: SyncMode) -> i32 {
    match mode {
        SyncMode::AddOnly => 0,
        SyncMode::Mirror => 1,
        SyncMode::Archive => 2,
    }
}

fn connection_id(
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
