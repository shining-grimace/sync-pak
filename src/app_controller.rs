use std::rc::Rc;

use slint::ComponentHandle;

use crate::{
    AppWindow,
    configuration::{ConfigStore, StructuredError},
    diagnostics_controller,
};

pub(crate) fn initialize(window: &AppWindow) {
    window.set_app_version(env!("CARGO_PKG_VERSION").into());
    let diagnostics = Rc::new(std::cell::RefCell::new(Default::default()));
    diagnostics_controller::configure(window, Rc::clone(&diagnostics));
    let configuration = match ConfigStore::for_current_platform() {
        Ok(configuration) => Rc::new(configuration),
        Err(_) => {
            diagnostics_controller::present(
                window,
                &diagnostics,
                "Configuration could not be opened",
                "configuration directory unavailable",
                "SyncPak could not access its configuration. Check its storage location and try again.",
            );
            return;
        }
    };
    configure_navigation(window);
    let provider_buckets: crate::provider_bucket_cache::ProviderBucketCache = Default::default();
    crate::provider_list_controller::configure(
        window,
        &configuration,
        Rc::clone(&diagnostics),
        Rc::clone(&provider_buckets),
    );
    crate::provider_form_controller::configure(
        window,
        &configuration,
        Rc::clone(&diagnostics),
        Rc::clone(&provider_buckets),
    );
    crate::provider_delete_controller::configure(
        window,
        &configuration,
        Rc::clone(&diagnostics),
        Rc::clone(&provider_buckets),
    );
    crate::connection_list_controller::configure(window, &configuration, Rc::clone(&diagnostics));
    crate::connection_form_controller::configure(
        window,
        &configuration,
        Rc::clone(&diagnostics),
        Rc::clone(&provider_buckets),
    );
    crate::connection_delete_controller::configure(window, &configuration, Rc::clone(&diagnostics));
    crate::run_direction_controller::configure(window, &configuration, Rc::clone(&diagnostics));
    crate::folder_picker_controller::configure(window, Rc::clone(&diagnostics));
    match configuration.load() {
        Ok(config) => {
            let report = crate::temporary_cleanup::remove_stale_files(
                config
                    .connections
                    .iter()
                    .map(|connection| &connection.local_path),
            );
            if !report.failures.is_empty() {
                diagnostics_controller::record(
                    &diagnostics,
                    StructuredError::new(
                        "Could not remove temporary data from an earlier operation",
                        "startup temporary-file cleanup failed",
                    ),
                );
            }
            if config.welcome_completed {
                crate::provider_list_controller::show(
                    &window.as_weak(),
                    configuration,
                    diagnostics,
                    provider_buckets,
                )
            }
        }
        Err(_) => diagnostics_controller::present(
            window,
            &diagnostics,
            "Configuration could not be loaded",
            "configuration load failed",
            "SyncPak could not load its configuration. Check the file and try again.",
        ),
    }
}

fn configure_navigation(window: &AppWindow) {
    let weak = window.as_weak();
    window.on_show_welcome(move || set_page(&weak, 0));
    let weak = window.as_weak();
    window.on_show_privacy(move || set_page(&weak, 3));
    let weak = window.as_weak();
    window.on_show_activity(move || set_page(&weak, 9));

    let weak = window.as_weak();
    window.on_request_navigation(move |page| request_navigation(&weak, page));

    let weak = window.as_weak();
    window.on_complete_pending_navigation(move || complete_pending_navigation(&weak));
}

fn set_page(weak: &slint::Weak<AppWindow>, page: i32) {
    if let Some(window) = weak.upgrade() {
        window.set_status_message(Default::default());
        window.set_notice_message(Default::default());
        window.set_page(page);
    }
}

fn request_navigation(weak: &slint::Weak<AppWindow>, page: i32) {
    let Some(window) = weak.upgrade() else { return };
    match window.get_page() {
        2 => {
            window.set_pending_navigation_page(page);
            window.invoke_request_discard_provider();
        }
        5 => {
            window.set_pending_navigation_page(page);
            window.invoke_request_discard_connection();
        }
        _ => navigate(&window, page),
    }
}

fn complete_pending_navigation(weak: &slint::Weak<AppWindow>) {
    let Some(window) = weak.upgrade() else { return };
    let page = match window.get_pending_navigation_page() {
        page if page >= 0 => page,
        _ if window.get_page() == 2 || window.get_page() == 15 => 1,
        _ => 4,
    };
    window.set_pending_navigation_page(-1);
    navigate(&window, page);
}

fn navigate(window: &AppWindow, page: i32) {
    match page {
        1 => window.invoke_show_providers(),
        3 => window.invoke_show_privacy(),
        4 => window.invoke_show_connections(),
        9 => window.invoke_show_activity(),
        _ => {}
    }
}
