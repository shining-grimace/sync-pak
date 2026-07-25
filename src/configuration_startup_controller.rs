use std::rc::Rc;

use slint::ComponentHandle;

use crate::{
    AppWindow,
    configuration::{ConfigStore, StructuredError},
    diagnostics_controller,
};

pub(crate) fn configure(
    window: &AppWindow,
    diagnostics: diagnostics_controller::SharedDiagnosticLog,
) {
    let configuration = match ConfigStore::for_current_platform() {
        Ok(configuration) => Rc::new(configuration),
        Err(_) => {
            show_unavailable(
                window,
                diagnostics,
                "Configuration could not be opened",
                "configuration directory unavailable",
                "SyncPak could not access its configuration. Check its storage location and try again.",
            );
            return;
        }
    };
    window.set_configuration_unavailable(false);
    window.set_status_message(Default::default());
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
            record_temporary_cleanup_failures(&config, &diagnostics);
            if config.welcome_completed {
                crate::provider_list_controller::show(
                    &window.as_weak(),
                    configuration,
                    diagnostics,
                    provider_buckets,
                )
            }
        }
        Err(_) => show_unavailable(
            window,
            diagnostics,
            "Configuration could not be loaded",
            "configuration load failed",
            "SyncPak could not load its configuration. Check the file and try again.",
        ),
    }
}

fn record_temporary_cleanup_failures(
    config: &crate::configuration::AppConfig,
    diagnostics: &diagnostics_controller::SharedDiagnosticLog,
) {
    let report = crate::temporary_cleanup::remove_stale_files(
        config
            .connections
            .iter()
            .map(|connection| &connection.local_path),
    );
    if !report.failures.is_empty() {
        diagnostics_controller::record(
            diagnostics,
            StructuredError::new(
                "Could not remove temporary data from an earlier operation",
                "startup temporary-file cleanup failed",
            ),
        );
    }
}

fn show_unavailable(
    window: &AppWindow,
    diagnostics: diagnostics_controller::SharedDiagnosticLog,
    summary: &'static str,
    technical_details: &'static str,
    message: &'static str,
) {
    window.set_configuration_unavailable(true);
    diagnostics_controller::present(window, &diagnostics, summary, technical_details, message);
    let weak = window.as_weak();
    window.on_retry_configuration(move || {
        if let Some(window) = weak.upgrade() {
            configure(&window, Rc::clone(&diagnostics));
        }
    });
}
