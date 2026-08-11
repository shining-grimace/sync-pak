use std::rc::Rc;

#[cfg(feature = "provider-s3")]
use std::sync::Arc;

use slint::ComponentHandle;

use crate::{
    AppWindow,
    app::diagnostics as diagnostics_controller,
    configuration::{ConfigStore, StructuredError},
};

pub(crate) fn configure(
    window: &AppWindow,
    diagnostics: crate::app::diagnostics::SharedDiagnosticLog,
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
    crate::app::preflight::controller::configure(window);
    let provider_buckets: crate::app::providers::bucket_cache::ProviderBucketCache =
        Default::default();
    crate::app::providers::list_controller::configure(
        window,
        &configuration,
        Rc::clone(&diagnostics),
        Rc::clone(&provider_buckets),
    );
    crate::app::providers::form::controller::configure(
        window,
        &configuration,
        Rc::clone(&diagnostics),
        Rc::clone(&provider_buckets),
    );
    crate::app::providers::form::secret_reveal::configure(window);
    crate::app::connections::list_controller::configure(
        window,
        &configuration,
        Rc::clone(&diagnostics),
    );
    crate::app::connections::form::controller::configure(
        window,
        &configuration,
        Rc::clone(&diagnostics),
        Rc::clone(&provider_buckets),
    );
    crate::app::run::direction_controller::configure(
        window,
        &configuration,
        Rc::clone(&diagnostics),
    );
    crate::app::folder_picker::configure(window, Rc::clone(&diagnostics));
    #[cfg(feature = "provider-s3")]
    {
        let queue = configure_operation_queue(window, &configuration);
        crate::app::providers::delete_controller::configure(
            window,
            &configuration,
            Rc::clone(&diagnostics),
            Rc::clone(&provider_buckets),
            Some(queue.clone()),
        );
        crate::app::connections::delete_controller::configure(
            window,
            &configuration,
            Rc::clone(&diagnostics),
            Some(queue),
        );
    }
    #[cfg(not(feature = "provider-s3"))]
    {
        crate::app::providers::delete_controller::configure(
            window,
            &configuration,
            Rc::clone(&diagnostics),
            Rc::clone(&provider_buckets),
            None,
        );
        crate::app::connections::delete_controller::configure(
            window,
            &configuration,
            Rc::clone(&diagnostics),
            None,
        );
    }
    match configuration.load() {
        Ok(config) => {
            crate::app::appearance::configure(&window, &configuration, config.appearance);
            record_temporary_cleanup_failures(&config, &diagnostics);
            if config.welcome_completed {
                crate::app::connections::list_controller::show(
                    &window.as_weak(),
                    configuration,
                    diagnostics,
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
) -> Arc<
    crate::operations::queue::background::BackgroundQueue<
        crate::providers::s3::operations::executor::S3OperationExecutor,
    >,
> {
    let executor = Arc::new(
        crate::providers::s3::operations::executor::S3OperationExecutor::new(
            configuration.path().to_owned(),
        ),
    );
    let queue = Arc::new(make_operation_queue(executor));
    crate::app::activity::controller::configure(window, Arc::clone(&queue));
    crate::app::run::start_controller::configure(window, Arc::clone(&queue));
    queue
}

#[cfg(all(feature = "provider-s3", target_os = "android"))]
fn make_operation_queue(
    executor: Arc<crate::providers::s3::operations::executor::S3OperationExecutor>,
) -> crate::operations::queue::background::BackgroundQueue<
    crate::providers::s3::operations::executor::S3OperationExecutor,
> {
    crate::operations::queue::background::BackgroundQueue::with_background_execution(
        executor,
        Arc::new(crate::platform::PlatformBackgroundExecution),
    )
}

#[cfg(all(feature = "provider-s3", not(target_os = "android")))]
fn make_operation_queue(
    executor: Arc<crate::providers::s3::operations::executor::S3OperationExecutor>,
) -> crate::operations::queue::background::BackgroundQueue<
    crate::providers::s3::operations::executor::S3OperationExecutor,
> {
    crate::operations::queue::background::BackgroundQueue::new(executor)
}

fn record_temporary_cleanup_failures(
    config: &crate::configuration::AppConfig,
    diagnostics: &crate::app::diagnostics::SharedDiagnosticLog,
) {
    let report = crate::platform::temporary_cleanup::remove_stale_files(
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
    diagnostics: crate::app::diagnostics::SharedDiagnosticLog,
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
