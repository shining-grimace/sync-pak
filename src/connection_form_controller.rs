use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use crate::{
    AppWindow,
    configuration::{ConfigStore, ConnectionRepository},
    connection_form_data::{clear_transient_state, draft, existing_id, is_dirty},
    connection_form_state, connection_list_controller,
    diagnostics_controller::{self, SharedDiagnosticLog},
    form_validation,
    provider_bucket_cache::ProviderBucketCache,
};

pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    buckets: ProviderBucketCache,
) {
    let weak = window.as_weak();
    let add_configuration = Rc::clone(configuration);
    let add_diagnostics = Rc::clone(&diagnostics);
    let add_buckets = Rc::clone(&buckets);
    window.on_show_add_connection(move || {
        connection_form_state::show_add(
            &weak,
            Rc::clone(&add_configuration),
            Rc::clone(&add_diagnostics),
            Rc::clone(&add_buckets),
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
    let edit_buckets = Rc::clone(&buckets);
    window.on_request_connection_edit(move |id| {
        connection_form_state::show_edit(
            &weak,
            &edit_configuration,
            &edit_diagnostics,
            &edit_buckets,
            id,
        )
    });

    let weak = window.as_weak();
    let provider_configuration = Rc::clone(configuration);
    let provider_diagnostics = Rc::clone(&diagnostics);
    let provider_buckets = Rc::clone(&buckets);
    window.on_select_connection_provider(move |index| {
        connection_form_state::select_provider(
            &weak,
            &provider_configuration,
            &provider_diagnostics,
            &provider_buckets,
            index,
        )
    });

    let weak = window.as_weak();
    let retry_configuration = Rc::clone(configuration);
    let retry_diagnostics = Rc::clone(&diagnostics);
    let retry_buckets = Rc::clone(&buckets);
    window.on_retry_connection_providers(move || {
        connection_form_state::retry_load_providers(
            &weak,
            Rc::clone(&retry_configuration),
            Rc::clone(&retry_diagnostics),
            Rc::clone(&retry_buckets),
        )
    });

    let weak = window.as_weak();
    window.on_request_discard_connection(move || request_discard(&weak));

    let weak = window.as_weak();
    window.on_cancel_discard_connection(move || {
        if let Some(window) = weak.upgrade() {
            window.set_pending_navigation_page(-1);
            window.set_page(5);
        }
    });

    let weak = window.as_weak();
    window.on_confirm_discard_connection(move || {
        if let Some(window) = weak.upgrade() {
            clear_transient_state(&window);
            window.invoke_complete_pending_navigation();
        }
    });
}

fn request_discard(weak: &slint::Weak<AppWindow>) {
    let Some(window) = weak.upgrade() else { return };
    if window.get_pending_navigation_page() < 0 {
        window.set_pending_navigation_page(4);
    }
    if is_dirty(&window) {
        window.set_page(14);
    } else {
        clear_transient_state(&window);
        window.invoke_complete_pending_navigation();
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
        Ok(_) => {
            clear_transient_state(&window);
            connection_list_controller::show(weak, configuration, diagnostics);
            window.set_notice_message("Connection saved.".into());
        }
        Err(_) => diagnostics_controller::present(
            &window,
            &diagnostics,
            "Connection could not be saved",
            "connection save failed",
            "SyncPak could not save this connection. Check configuration storage and try again.",
        ),
    }
}
