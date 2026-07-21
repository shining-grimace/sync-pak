use std::{rc::Rc, time::Duration};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{
    AppWindow, ConnectionRow,
    configuration::{ConfigStore, SyncMode},
    diagnostics_controller::{self, SharedDiagnosticLog},
};

pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let weak = window.as_weak();
    let show_configuration = Rc::clone(configuration);
    let show_diagnostics = Rc::clone(&diagnostics);
    window.on_show_connections(move || {
        if let Some(window) = weak.upgrade() {
            window.set_connection_filter(0);
        }
        show(
            &weak,
            Rc::clone(&show_configuration),
            Rc::clone(&show_diagnostics),
        )
    });

    let weak = window.as_weak();
    let configuration = Rc::clone(configuration);
    window.on_set_connection_filter(move |filter| {
        if let Some(window) = weak.upgrade() {
            window.set_connection_filter(filter.clamp(0, 3));
        }
        refresh(&weak, &configuration, &diagnostics);
    });
}

pub(crate) fn show(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    window.set_status_message(SharedString::default());
    window.set_page(4);
    let weak = weak.clone();
    slint::Timer::single_shot(Duration::ZERO, move || {
        refresh(&weak, &configuration, &diagnostics)
    });
}

fn refresh(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    match configuration.load() {
        Ok(config) => {
            window.set_connections_total(config.connections.len() as i32);
            let filter = window.get_connection_filter();
            let rows = config
                .connections
                .iter()
                .filter(|connection| filter == 0 || mode_index(connection.mode) == filter - 1)
                .map(|connection| {
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
                        mode: mode_index(connection.mode),
                        local_path: connection.local_path.clone().into(),
                        provider_name: provider.into(),
                        archive_retention: connection
                            .keep_last_archives
                            .map_or_else(String::new, |retention| retention.to_string())
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

fn mode_index(mode: SyncMode) -> i32 {
    match mode {
        SyncMode::AddOnly => 0,
        SyncMode::Mirror => 1,
        SyncMode::Archive => 2,
    }
}

fn mode_name(mode: SyncMode) -> &'static str {
    match mode {
        SyncMode::AddOnly => "Add-only",
        SyncMode::Mirror => "Mirror",
        SyncMode::Archive => "Archive",
    }
}
