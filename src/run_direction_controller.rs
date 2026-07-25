use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use crate::{
    AppWindow,
    configuration::{ConfigStore, SyncMode},
    connection_list_controller,
    diagnostics_controller::{self, SharedDiagnosticLog},
    planning::Direction,
    run_direction_presentation::{archive_details, mode_label, remote_endpoint},
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
        if let Some(window) = weak.upgrade() {
            crate::preflight_controller::invalidate(&window);
        }
        connection_list_controller::show(&weak, Rc::clone(&configuration), Rc::clone(&diagnostics));
    });

    let weak = window.as_weak();
    window.on_return_to_run_direction(move || {
        let Some(window) = weak.upgrade() else { return };
        crate::preflight_controller::invalidate(&window);
        window.set_page(10);
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
            let generation = window.get_preflight_generation();
            start_preflight(
                weak.clone(),
                request,
                configuration.path().to_owned(),
                Rc::clone(diagnostics),
                generation,
            );
        }
        Err(_) => {
            crate::preflight_controller::show_failed(
                &window,
                "SyncPak could not prepare this connection. Check that it and its provider still exist.",
            );
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
    generation: i32,
) {
    crate::s3_preflight_controller::start(
        weak,
        request,
        configuration_path,
        diagnostics,
        generation,
    );
}

#[cfg(not(feature = "provider-s3"))]
fn start_preflight(
    weak: slint::Weak<AppWindow>,
    _: RunRequest,
    _: std::path::PathBuf,
    diagnostics: SharedDiagnosticLog,
    _: i32,
) {
    let Some(window) = weak.upgrade() else { return };
    crate::preflight_controller::show_failed(
        &window,
        "This SyncPak build cannot connect to cloud storage. Install a build with provider support and try again.",
    );
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
    let run = configuration.load().ok().and_then(|config| {
        config
            .connections
            .iter()
            .find(|item| item.id.as_str() == id.as_str())
            .cloned()
            .map(|connection| {
                let provider_name = config
                    .providers
                    .iter()
                    .find(|provider| provider.id == connection.provider_id)
                    .map_or("Unavailable provider", |provider| provider.name.as_str());
                let remote_endpoint = remote_endpoint(provider_name, &connection);
                (connection, remote_endpoint)
            })
    });
    match run {
        Some((connection, remote_endpoint)) => {
            let archive_upload_details = archive_details(&connection, Direction::Upload);
            let archive_download_details = archive_details(&connection, Direction::Download);
            window.set_status_message(SharedString::default());
            window.set_run_connection_id(connection.id.as_str().into());
            window.set_run_connection_name(connection.name.into());
            window.set_run_connection_mode(mode_label(connection.mode).into());
            window.set_run_local_endpoint(connection.local_path.into());
            window.set_run_remote_endpoint(remote_endpoint.into());
            window.set_run_archive_upload_details(archive_upload_details.into());
            window.set_run_archive_download_details(archive_download_details.into());
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

#[cfg(test)]
#[path = "run_direction_controller_tests.rs"]
mod tests;
