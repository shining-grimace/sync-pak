use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use crate::{
    AppWindow, ConfirmationKind,
    app::connections::list_controller as connection_list_controller,
    app::diagnostics::{self as diagnostics_controller, SharedDiagnosticLog},
    app::form_validation,
    configuration::{ConfigStore, ConnectionDraft, ConnectionRepository},
};

use super::data::{clear_transient_state, draft, existing_id, is_dirty, was_verified};

#[allow(clippy::too_many_arguments)]
pub(super) fn save(
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
    allow_upload: bool,
    allow_download: bool,
) {
    let Some(window) = weak.upgrade() else { return };
    if window.get_connection_verifying() || window.get_confirmation() != ConfirmationKind::None {
        return;
    }
    if let Err(error) = form_validation::directions(allow_upload, allow_download) {
        window.set_status_message(error.into());
        return;
    }
    if let Err(error) = form_validation::connection(
        &name,
        provider_index,
        &bucket,
        &local_path,
        mode_index,
        &retention,
    ) {
        window.set_connection_validation_focus(form_validation::connection_focus(&error));
        window.set_status_message(error.into());
        return;
    }
    let transport = match super::transport::read(&window, &configuration) {
        Ok(settings) => settings,
        Err(error) => {
            window.set_status_message(error.into());
            return;
        }
    };
    let edit_id = window.get_connection_form_id();
    let form_changed = is_dirty(&window);
    let connection = draft(
        &configuration,
        transport,
        name,
        provider_index,
        bucket,
        remote_path,
        local_path,
        mode_index,
        retention,
        allow_upload,
        allow_download,
        window.get_connection_form_show_preview(),
    )
    .map(|mut draft| {
        draft.verified = !form_changed && was_verified(&configuration, edit_id.as_str());
        draft
    });
    persist(&window, configuration, &diagnostics, connection);
}

pub(super) fn persist(
    window: &AppWindow,
    configuration: Rc<ConfigStore>,
    diagnostics: &SharedDiagnosticLog,
    connection: Result<ConnectionDraft, String>,
) {
    let edit_id = window.get_connection_form_id();
    let result = connection.and_then(|draft| {
        let repository = ConnectionRepository::new(&configuration);
        if edit_id.is_empty() {
            repository.create(draft).map_err(|error| error.to_string())
        } else {
            repository
                .update(&existing_id(&configuration, edit_id.as_str())?, draft)
                .map_err(|error| error.to_string())
        }
    });
    match result {
        Ok(_) => {
            clear_transient_state(window);
            connection_list_controller::show(
                &window.as_weak(),
                configuration,
                Rc::clone(diagnostics),
            );
            window.set_notice_message("Connection saved.".into());
        }
        Err(error) => {
            diagnostics_controller::present(
                window,
                diagnostics,
                "Connection could not be saved",
                "connection save failed",
                "SyncPak could not save this connection.",
            );
            window.set_status_message(error.into());
        }
    }
}
