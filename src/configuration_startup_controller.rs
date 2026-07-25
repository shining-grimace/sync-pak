use std::rc::Rc;

#[cfg(feature = "provider-s3")]
use std::sync::Arc;

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
    crate::preflight_controller::configure(window);
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
    crate::provider_secret_reveal_controller::configure(window);
    crate::connection_list_controller::configure(window, &configuration, Rc::clone(&diagnostics));
    crate::connection_form_controller::configure(
        window,
        &configuration,
        Rc::clone(&diagnostics),
        Rc::clone(&provider_buckets),
    );
    crate::run_direction_controller::configure(window, &configuration, Rc::clone(&diagnostics));
    crate::folder_picker_controller::configure(window, Rc::clone(&diagnostics));
    #[cfg(feature = "provider-s3")]
    {
        let queue = configure_operation_queue(window, &configuration);
        crate::provider_delete_controller::configure(
            window,
            &configuration,
            Rc::clone(&diagnostics),
            Rc::clone(&provider_buckets),
            Some(queue.clone()),
        );
        crate::connection_delete_controller::configure(
            window,
            &configuration,
            Rc::clone(&diagnostics),
            Some(queue),
        );
    }
    #[cfg(not(feature = "provider-s3"))]
    {
        crate::provider_delete_controller::configure(
            window,
            &configuration,
            Rc::clone(&diagnostics),
            Rc::clone(&provider_buckets),
            None,
        );
        crate::connection_delete_controller::configure(
            window,
            &configuration,
            Rc::clone(&diagnostics),
            None,
        );
    }
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

#[cfg(feature = "provider-s3")]
fn configure_operation_queue(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
) -> Arc<crate::background_queue::BackgroundQueue<crate::s3_operation_executor::S3OperationExecutor>>
{
    let executor = Arc::new(crate::s3_operation_executor::S3OperationExecutor::new(
        configuration.path().to_owned(),
    ));
    let queue = Arc::new(make_operation_queue(executor));
    crate::activity_controller::configure(window, Arc::clone(&queue));
    crate::operation_start_controller::configure(window, Arc::clone(&queue));
    queue
}

#[cfg(all(feature = "provider-s3", target_os = "android"))]
fn make_operation_queue(
    executor: Arc<crate::s3_operation_executor::S3OperationExecutor>,
) -> crate::background_queue::BackgroundQueue<crate::s3_operation_executor::S3OperationExecutor> {
    crate::background_queue::BackgroundQueue::with_background_execution(
        executor,
        Arc::new(crate::platform::PlatformBackgroundExecution),
    )
}

#[cfg(all(feature = "provider-s3", not(target_os = "android")))]
fn make_operation_queue(
    executor: Arc<crate::s3_operation_executor::S3OperationExecutor>,
) -> crate::background_queue::BackgroundQueue<crate::s3_operation_executor::S3OperationExecutor> {
    crate::background_queue::BackgroundQueue::new(executor)
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
