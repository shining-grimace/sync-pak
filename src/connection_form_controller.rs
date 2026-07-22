use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use crate::{
    AppWindow,
    configuration::{ConfigStore, ConnectionRepository},
    connection_form_data::{draft, existing_id, is_dirty, populate},
    connection_form_state, connection_list_controller,
    diagnostics_controller::{self, SharedDiagnosticLog},
    form_validation,
};

pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let weak = window.as_weak();
    let add_configuration = Rc::clone(configuration);
    let add_diagnostics = Rc::clone(&diagnostics);
    window.on_show_add_connection(move || {
        connection_form_state::show_add(
            &weak,
            Rc::clone(&add_configuration),
            Rc::clone(&add_diagnostics),
        )
    });

    let weak = window.as_weak();
    let save_configuration = Rc::clone(configuration);
    let save_diagnostics = Rc::clone(&diagnostics);
    window.on_save_connection(
        move |name, provider, bucket, remote, local, mode, retention| {
            save(
                &weak,
                Rc::clone(&save_configuration),
                Rc::clone(&save_diagnostics),
                name,
                provider,
                bucket,
                remote,
                local,
                mode,
                retention,
            )
        },
    );

    let weak = window.as_weak();
    let edit_configuration = Rc::clone(configuration);
    let edit_diagnostics = Rc::clone(&diagnostics);
    window.on_request_connection_edit(move |id| {
        edit(&weak, &edit_configuration, &edit_diagnostics, id)
    });

    let weak = window.as_weak();
    let provider_configuration = Rc::clone(configuration);
    let provider_diagnostics = Rc::clone(&diagnostics);
    window.on_select_connection_provider(move |index| {
        connection_form_state::select_provider(
            &weak,
            &provider_configuration,
            &provider_diagnostics,
            index,
        )
    });

    let weak = window.as_weak();
    let discard_configuration = Rc::clone(configuration);
    let discard_diagnostics = Rc::clone(&diagnostics);
    window.on_request_discard_connection(move || {
        request_discard(
            &weak,
            Rc::clone(&discard_configuration),
            Rc::clone(&discard_diagnostics),
        );
    });

    let weak = window.as_weak();
    window.on_cancel_discard_connection(move || {
        if let Some(window) = weak.upgrade() {
            window.set_page(5);
        }
    });

    let weak = window.as_weak();
    let discard_configuration = Rc::clone(configuration);
    window.on_confirm_discard_connection(move || {
        connection_list_controller::show(
            &weak,
            Rc::clone(&discard_configuration),
            Rc::clone(&diagnostics),
        );
    });
}

fn request_discard(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    if is_dirty(&window) {
        window.set_page(14);
    } else {
        connection_list_controller::show(weak, configuration, diagnostics);
    }
}

#[allow(clippy::too_many_arguments)]
fn save(
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
    let result = draft(
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
            repository
                .update(&existing_id(&configuration, edit_id.as_str())?, draft)
                .map_err(|error| error.to_string())
        }
    });
    match result {
        Ok(_) => connection_list_controller::show(weak, configuration, diagnostics),
        Err(_) => diagnostics_controller::present(
            &window,
            &diagnostics,
            "Connection could not be saved",
            "connection save failed",
            "SyncPak could not save this connection. Check configuration storage and try again.",
        ),
    }
}

fn edit(
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
            populate(&window, &providers, connection, provider_index)
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
