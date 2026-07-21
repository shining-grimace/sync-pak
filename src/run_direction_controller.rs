use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use crate::{
    AppWindow,
    configuration::{ConfigStore, SyncMode},
    connection_list_controller,
    diagnostics_controller::{self, SharedDiagnosticLog},
};

/// Presents direction choices for a saved connection before its preflight begins.
pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let weak = window.as_weak();
    let run_configuration = Rc::clone(configuration);
    let diagnostics_for_run = Rc::clone(&diagnostics);
    window.on_request_run_connection(move |id| {
        show(&weak, &run_configuration, &diagnostics_for_run, id);
    });

    let weak = window.as_weak();
    window.on_choose_run_direction(move |direction| {
        if let Some(window) = weak.upgrade() {
            window.set_run_direction(direction.clamp(0, 2));
        }
    });

    let weak = window.as_weak();
    window.on_begin_preflight(move || {
        if let Some(window) = weak.upgrade() {
            window.set_status_message(SharedString::default());
            window.set_page(11);
        }
    });

    let weak = window.as_weak();
    let configuration = Rc::clone(configuration);
    window.on_cancel_run_direction(move || {
        connection_list_controller::show(&weak, Rc::clone(&configuration), Rc::clone(&diagnostics));
    });
}

fn show(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
    id: SharedString,
) {
    let Some(window) = weak.upgrade() else { return };
    let connection = configuration.load().ok().and_then(|config| {
        config
            .connections
            .into_iter()
            .find(|item| item.id.as_str() == id.as_str())
    });
    match connection {
        Some(connection) => {
            window.set_status_message(SharedString::default());
            window.set_run_connection_id(connection.id.as_str().into());
            window.set_run_connection_name(connection.name.into());
            window.set_run_connection_mode(mode_label(connection.mode).into());
            window.set_run_allows_both_ways(connection.mode == SyncMode::AddOnly);
            window.set_run_direction(0);
            window.set_page(10);
        }
        None => diagnostics_controller::present(
            &window,
            diagnostics,
            "Connection could not be opened",
            "run connection load failed",
            "SyncPak could not open this connection. It may have been removed.",
        ),
    }
}

fn mode_label(mode: SyncMode) -> &'static str {
    match mode {
        SyncMode::AddOnly => "Add-only",
        SyncMode::Mirror => "Mirror",
        SyncMode::Archive => "Archive",
    }
}
