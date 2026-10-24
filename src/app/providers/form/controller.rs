use super::{
    edit::edit,
    save::save,
    state::{clear_transient_state, invalidate_verification, is_dirty, mark_clean},
    verification::verify,
};
use crate::{
    AppWindow,
    app::{diagnostics::SharedDiagnosticLog, providers::bucket_cache::ProviderBucketCache},
    configuration::ConfigStore,
};
use slint::{ComponentHandle, SharedString};
use std::rc::Rc;

pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    buckets: ProviderBucketCache,
) {
    window.on_infer_b2_region(|value| {
        crate::providers::endpoint::b2_region(&value)
            .unwrap_or_default()
            .into()
    });
    let weak = window.as_weak();
    window.on_show_add_provider(move || show_add(&weak));

    let weak = window.as_weak();
    let save_configuration = Rc::clone(configuration);
    let save_diagnostics = Rc::clone(&diagnostics);
    let save_buckets = Rc::clone(&buckets);
    window.on_save_provider(move |name, kind, _, _| {
        save(
            &weak,
            Rc::clone(&save_configuration),
            &save_diagnostics,
            &save_buckets,
            name,
            kind,
        )
    });

    let weak = window.as_weak();
    let verify_configuration = Rc::clone(configuration);
    let verify_diagnostics = Rc::clone(&diagnostics);
    window.on_verify_provider(move || {
        if let Some(window) = weak.upgrade() {
            if window.get_provider_storage_busy() || window.get_provider_verifying() {
                return;
            }
            window.set_provider_save_after_verification(false);
        }
        verify(
            &weak,
            Rc::clone(&verify_configuration),
            Rc::clone(&verify_diagnostics),
        );
    });

    let weak = window.as_weak();
    let save_and_verify_configuration = Rc::clone(configuration);
    let save_and_verify_diagnostics = Rc::clone(&diagnostics);
    window.on_save_and_verify_provider(move || {
        if let Some(window) = weak.upgrade() {
            if window.get_provider_storage_busy() || window.get_provider_verifying() {
                return;
            }
            window.set_provider_save_after_verification(true);
        }
        verify(
            &weak,
            Rc::clone(&save_and_verify_configuration),
            Rc::clone(&save_and_verify_diagnostics),
        );
    });

    let weak = window.as_weak();
    window.on_request_save_provider(move || {
        if let Some(window) = weak.upgrade() {
            crate::app::providers::form::secret_reveal::hide(&window);
            window.set_status_message(SharedString::default());
            window.set_page(13);
        }
    });

    let weak = window.as_weak();
    window.on_cancel_save_provider(move || {
        if let Some(window) = weak.upgrade() {
            if window.get_provider_storage_busy() {
                return;
            }
            window.set_page(2);
        }
    });

    let weak = window.as_weak();
    window.on_request_discard_provider(move || request_discard(&weak));

    let weak = window.as_weak();
    window.on_cancel_discard_provider(move || {
        if let Some(window) = weak.upgrade() {
            window.set_pending_navigation_page(-1);
            window.set_page(2);
        }
    });

    let weak = window.as_weak();
    window.on_confirm_discard_provider(move || {
        if let Some(window) = weak.upgrade() {
            clear_transient_state(&window);
            window.invoke_complete_pending_navigation();
        }
    });

    let weak = window.as_weak();
    let edit_configuration = Rc::clone(configuration);
    window.on_request_provider_edit(move |id| {
        if let Some(window) = weak.upgrade()
            && window.get_busy_provider_id() == id
            && !id.is_empty()
        {
            window.set_status_message("This provider is in use by a running operation.".into());
            return;
        }
        edit(&weak, &edit_configuration, &diagnostics, id)
    });
}

fn show_add(weak: &slint::Weak<AppWindow>) {
    if let Some(window) = weak.upgrade() {
        if window.get_provider_storage_busy() {
            return;
        }
        window.set_status_message(SharedString::default());
        window.set_provider_form_id(SharedString::default());
        window.set_provider_form_name(SharedString::default());
        window.set_provider_form_kind(0);
        window.set_provider_form_account_id(SharedString::default());
        window.set_provider_form_region(SharedString::default());
        window.set_provider_form_bucket(SharedString::default());
        window.set_provider_form_endpoint(SharedString::default());
        super::transport::reset(&window);
        clear_transient_state(&window);
        mark_clean(&window);
        window.set_page(2);
    }
}

fn request_discard(weak: &slint::Weak<AppWindow>) {
    let Some(window) = weak.upgrade() else { return };
    if window.get_provider_storage_busy() {
        return;
    }
    crate::app::providers::form::secret_reveal::hide(&window);
    invalidate_verification(&window);
    if window.get_pending_navigation_page() < 0 {
        window.set_pending_navigation_page(1);
    }
    if is_dirty(&window) {
        window.set_page(15);
    } else {
        clear_transient_state(&window);
        window.invoke_complete_pending_navigation();
    }
}
