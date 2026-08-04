use std::{rc::Rc, time::Duration};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{
    AppWindow, ConnectionRow,
    configuration::{ConfigStore, SyncMode},
    connection_list_verification_controller::{self, SessionVerifications, VerificationStates},
    diagnostics_controller::{self, SharedDiagnosticLog},
};

pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let states: VerificationStates = Default::default();
    let sessions: SessionVerifications = Default::default();
    let weak = window.as_weak();
    let show_configuration = Rc::clone(configuration);
    let show_diagnostics = Rc::clone(&diagnostics);
    let show_states = Rc::clone(&states);
    let show_sessions = Rc::clone(&sessions);
    window.on_show_connections(move || {
        if let Some(window) = weak.upgrade() {
            window.set_connection_filter(0);
        }
        show_with_states(
            &weak,
            Rc::clone(&show_configuration),
            Rc::clone(&show_diagnostics),
            Rc::clone(&show_states),
            Rc::clone(&show_sessions),
        )
    });

    let weak = window.as_weak();
    let filter_configuration = Rc::clone(configuration);
    let filter_diagnostics = Rc::clone(&diagnostics);
    let filter_states = Rc::clone(&states);
    let filter_sessions = Rc::clone(&sessions);
    window.on_set_connection_filter(move |filter| {
        if let Some(window) = weak.upgrade() {
            window.set_connection_filter(filter.clamp(0, 3));
        }
        refresh(
            &weak,
            &filter_configuration,
            &filter_diagnostics,
            &filter_states,
            &filter_sessions,
        );
    });

    let weak = window.as_weak();
    let verify_configuration = Rc::clone(configuration);
    window.on_verify_saved_connection(move |id| {
        connection_list_verification_controller::verify(
            &weak,
            Rc::clone(&verify_configuration),
            Rc::clone(&diagnostics),
            Rc::clone(&states),
            Rc::clone(&sessions),
            id.to_string(),
        );
    });
}

pub(crate) fn show(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    show_with_states(
        weak,
        configuration,
        diagnostics,
        Default::default(),
        Default::default(),
    );
}

fn show_with_states(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    states: VerificationStates,
    sessions: SessionVerifications,
) {
    let Some(window) = weak.upgrade() else { return };
    window.set_status_message(SharedString::default());
    window.set_notice_message(SharedString::default());
    window.set_page(4);
    let weak = weak.clone();
    slint::Timer::single_shot(Duration::ZERO, move || {
        refresh(&weak, &configuration, &diagnostics, &states, &sessions)
    });
}

pub(crate) fn refresh(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
    states: &VerificationStates,
    sessions: &SessionVerifications,
) {
    let Some(window) = weak.upgrade() else { return };
    match configuration.load() {
        Ok(config) => {
            window.set_connections_load_failed(false);
            window.set_connections_total(config.connections.len() as i32);
            window.set_providers_total(config.providers.len() as i32);
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
                        detail: mode_name(connection.mode).into(),
                        verification: connection_list_verification_controller::status(
                            states,
                            sessions,
                            connection.id.as_str(),
                            connection.verified,
                        )
                        .into(),
                        verifying: connection_list_verification_controller::is_checking(
                            states,
                            connection.id.as_str(),
                        ),
                        mode: mode_index(connection.mode),
                        local_path: connection.local_path.clone().into(),
                        provider_name: provider.into(),
                        remote_location: remote_location(
                            &connection.bucket,
                            &connection.remote_path,
                        )
                        .into(),
                        archive_retention: connection
                            .keep_last_archives
                            .map_or_else(String::new, |retention| retention.to_string())
                            .into(),
                    }
                });
            window.set_connections(ModelRc::new(Rc::new(VecModel::from_iter(rows))));
            window.set_status_message(SharedString::default());
        }
        Err(_) => {
            window.set_connections_load_failed(true);
            window.set_connections_total(0);
            window.set_providers_total(0);
            window.set_connections(ModelRc::new(Rc::new(VecModel::default())));
            diagnostics_controller::present(
                &window,
                diagnostics,
                "Connections could not be loaded",
                "connection configuration load failed",
                "SyncPak could not load connections. Check configuration storage and try again.",
            );
        }
    }
}

fn remote_location(bucket: &str, remote_path: &str) -> String {
    if remote_path.is_empty() {
        bucket.into()
    } else {
        format!("{bucket}/{remote_path}")
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
