use std::rc::Rc;

use slint::ComponentHandle;

use crate::{
    AppWindow, ConfirmationKind,
    app::connections::form::data::{clear_transient_state, invalidate_verification, is_dirty},
    app::connections::form::state as connection_form_state,
    app::connections::form::verification_controller as connection_form_verify_controller,
    app::diagnostics::SharedDiagnosticLog,
    app::providers::bucket_cache::ProviderBucketCache,
    configuration::ConfigStore,
};

pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    buckets: ProviderBucketCache,
) {
    super::transport::configure(window, configuration);
    connection_form_verify_controller::configure(window, configuration, Rc::clone(&diagnostics));
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
        move |name,
              provider,
              bucket,
              remote,
              local,
              mode,
              retention,
              allow_upload,
              allow_download| {
            super::save::save(
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
                allow_upload,
                allow_download,
            )
        },
    );

    let weak = window.as_weak();
    let edit_configuration = Rc::clone(configuration);
    let edit_diagnostics = Rc::clone(&diagnostics);
    let edit_buckets = Rc::clone(&buckets);
    window.on_request_connection_edit(move |id| {
        if let Some(window) = weak.upgrade()
            && window.get_busy_connection_id() == id
            && !id.is_empty()
        {
            window.set_status_message("This connection is in use by a running operation.".into());
            return;
        }
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
            window.set_confirmation(ConfirmationKind::None);
        }
    });

    let weak = window.as_weak();
    window.on_confirm_discard_connection(move || {
        if let Some(window) = weak.upgrade() {
            window.set_confirmation(ConfirmationKind::None);
            clear_transient_state(&window);
            window.invoke_complete_pending_navigation();
        }
    });
}

fn request_discard(weak: &slint::Weak<AppWindow>) {
    let Some(window) = weak.upgrade() else { return };
    if window.get_confirmation() != ConfirmationKind::None {
        return;
    }
    invalidate_verification(&window);
    if window.get_pending_navigation_page() < 0 {
        window.set_pending_navigation_page(4);
    }
    if is_dirty(&window) {
        window.set_confirmation(ConfirmationKind::DiscardConnection);
    } else {
        clear_transient_state(&window);
        window.invoke_complete_pending_navigation();
    }
}
