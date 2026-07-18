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
    let configuration = Rc::clone(configuration);
    window.on_show_connections(move || {
        show(&weak, Rc::clone(&configuration), Rc::clone(&diagnostics))
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

fn mode_name(mode: SyncMode) -> &'static str {
    match mode {
        SyncMode::AddOnly => "Add-only",
        SyncMode::Mirror => "Mirror",
        SyncMode::Archive => "Archive",
    }
}
