use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use crate::{
    AppWindow,
    configuration::{ConfigStore, SyncMode},
    connection_list_controller,
    diagnostics_controller::{self, SharedDiagnosticLog},
    planning::Direction,
    run_request::RunRequest,
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
    let preflight_configuration = Rc::clone(configuration);
    let preflight_diagnostics = Rc::clone(&diagnostics);
    window.on_begin_preflight(move || {
        begin_preflight(&weak, &preflight_configuration, &preflight_diagnostics);
    });

    let weak = window.as_weak();
    let configuration = Rc::clone(configuration);
    window.on_cancel_run_direction(move || {
        connection_list_controller::show(&weak, Rc::clone(&configuration), Rc::clone(&diagnostics));
    });
}

fn begin_preflight(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    let result = configuration.load().and_then(|config| {
        RunRequest::from_config(
            &config,
            window.get_run_connection_id().as_str(),
            direction(window.get_run_direction()),
        )
        .map_err(|error| crate::configuration::ConfigurationError::Io(std::io::Error::other(error)))
    });
    match result {
        Ok(request) => {
            window.set_status_message(SharedString::default());
            crate::preflight_controller::show_loading(&window);
            start_preflight(
                weak.clone(),
                request,
                configuration.path().to_owned(),
                Rc::clone(diagnostics),
            );
        }
        Err(_) => {
            crate::preflight_controller::show_failed(&window);
            diagnostics_controller::present(
                &window,
                diagnostics,
                "This operation cannot start",
                "run request validation failed",
                "SyncPak could not prepare this connection. Check that it and its provider still exist.",
            );
        }
    }
}

#[cfg(feature = "provider-s3")]
fn start_preflight(
    weak: slint::Weak<AppWindow>,
    request: RunRequest,
    configuration_path: std::path::PathBuf,
    diagnostics: SharedDiagnosticLog,
) {
    crate::s3_preflight_controller::start(weak, request, configuration_path, diagnostics);
}

#[cfg(not(feature = "provider-s3"))]
fn start_preflight(
    weak: slint::Weak<AppWindow>,
    _: RunRequest,
    _: std::path::PathBuf,
    diagnostics: SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    crate::preflight_controller::show_failed(&window);
    diagnostics_controller::present(
        &window,
        &diagnostics,
        "This operation cannot start",
        "S3 provider support is not enabled",
        "This SyncPak build cannot connect to cloud storage. Install a build with provider support and try again.",
    );
}

fn direction(index: i32) -> Direction {
    match index {
        1 => Direction::Download,
        2 => Direction::BothWays,
        _ => Direction::Upload,
    }
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
            let archive_details = archive_details(&connection);
            window.set_status_message(SharedString::default());
            window.set_run_connection_id(connection.id.as_str().into());
            window.set_run_connection_name(connection.name.into());
            window.set_run_connection_mode(mode_label(connection.mode).into());
            window.set_run_archive_details(archive_details.into());
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

fn archive_details(connection: &crate::configuration::ConnectionConfig) -> String {
    if connection.mode != SyncMode::Archive {
        return String::new();
    }
    let destination = if connection.remote_path.is_empty() {
        format!("the root of {}", connection.bucket)
    } else {
        format!("{}/{}", connection.bucket, connection.remote_path)
    };
    format!(
        "The new ZIP will be stored in {destination}. SyncPak will keep the newest {} archives.",
        connection.keep_last_archives.unwrap_or_default()
    )
}

fn mode_label(mode: SyncMode) -> &'static str {
    match mode {
        SyncMode::AddOnly => "Add-only",
        SyncMode::Mirror => "Mirror",
        SyncMode::Archive => "Archive",
    }
}
