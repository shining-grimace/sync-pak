#[allow(dead_code)]
mod activity_controller;
pub mod activity_presentation;
mod activity_progress_controller;
mod activity_result_controller;
pub mod activity_snapshot;
pub mod add_only_execution;
#[cfg(target_os = "android")]
mod android_folder_picker;
#[cfg(target_os = "android")]
mod android_foreground_execution;
#[cfg(target_os = "android")]
mod android_network_access;
mod app_controller;
mod appearance_controller;
pub mod archive_create;
mod archive_create_writer;
pub mod archive_download;
pub mod archive_download_store;
pub mod archive_execution;
pub mod archive_history;
pub mod archive_naming;
pub mod archive_prune;
pub mod archive_retention;
pub mod archive_store;
pub mod archive_upload;
pub mod atomic_write;
pub mod background_queue;
mod background_worker;
pub mod cancellation;
pub mod capabilities;
pub mod comparison;
pub mod configuration;
mod configuration_startup_controller;
pub mod confirmed_preflight;
mod connection_delete_controller;
mod connection_form_controller;
mod connection_form_data;
mod connection_form_state;
mod connection_form_verify_controller;
mod connection_list_controller;
mod connection_list_verification_controller;
pub mod connection_preflight;
mod connection_verification;
mod connection_verification_worker;
pub mod destructive_confirmation;
mod diagnostics_controller;
pub mod download;
pub mod execution;
#[cfg(test)]
mod feasibility;
pub mod filesystem;
mod folder_picker_controller;
mod form_validation;
pub mod inventory;
pub mod inventory_endpoint;
pub mod inventory_fingerprint;
pub mod local_archive_remover;
pub mod local_inventory;
pub mod local_remote_transfer;
mod local_remote_transfer_capabilities;
mod local_remote_transfer_download;
pub mod mirror_execution;
mod mirror_execution_error;
pub mod multipart_file_upload;
pub mod multipart_upload;
pub mod notifications;
mod onboarding;
pub mod operation_cancellation;
pub mod operation_progress;
#[cfg(feature = "provider-s3")]
mod operation_start_controller;
pub mod plan_summary;
pub mod planning;
pub mod platform;
pub mod preflight;
pub mod preflight_controller;
pub mod preflight_execution;
pub mod preflight_mirror_execution;
pub mod preflight_presentation;
pub mod preflight_review;
mod provider_bucket_cache;
pub mod provider_capabilities;
pub mod provider_conformance;
mod provider_delete_controller;
mod provider_form;
mod provider_form_controller;
mod provider_form_credentials;
mod provider_list_controller;
mod provider_list_verification_controller;
pub mod provider_multipart_conformance;
#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
mod provider_network_access;
#[cfg(feature = "provider-probes")]
pub mod provider_probe;
#[cfg(feature = "provider-probes")]
mod provider_probe_config;
mod provider_save_error;
mod provider_secret_reveal_controller;
pub mod provider_verification;
#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
mod provider_verification_failure;
#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
mod provider_verification_panic;
pub mod queue;
pub mod queue_progress_observer;
pub mod queue_retry_observer;
pub mod queue_runner;
pub mod remote_inventory;
pub mod retry;
pub mod reviewed_operation;
mod run_direction_controller;
mod run_direction_presentation;
pub mod run_request;
#[cfg(feature = "provider-s3")]
mod s3_archive_operation;
#[cfg(feature = "provider-s3")]
mod s3_bucket;
#[cfg(feature = "provider-s3")]
mod s3_error;
#[cfg(feature = "provider-s3")]
mod s3_multipart;
#[cfg(feature = "provider-s3")]
pub mod s3_operation_executor;
#[cfg(feature = "provider-s3")]
pub mod s3_preflight;
#[cfg(feature = "provider-s3")]
mod s3_preflight_controller;
#[cfg(feature = "provider-s3")]
pub mod s3_provider_verification;
#[cfg(feature = "provider-s3")]
mod s3_provider_verification_worker;
#[cfg(feature = "provider-s3")]
mod s3_provider_verify_controller;
#[cfg(feature = "provider-s3")]
mod s3_settings;
#[cfg(feature = "provider-s3")]
pub mod s3_transport;
#[cfg(feature = "provider-s3")]
mod s3_writer;
mod saved_provider_verification;
pub mod temporary_cleanup;
pub mod transfer_delete;
pub mod transfer_execution;
pub mod transfer_paths;
pub mod transfer_progress;
pub mod upload;
pub mod upload_strategy;

pub use capabilities::CapabilityError;

slint::include_modules!();

/// Opens the SyncPak application window and runs its event loop.
pub fn run() -> Result<(), slint::PlatformError> {
    let window = AppWindow::new()?;
    // The desktop backend resolves the native colour scheme when the window is
    // shown. Initialise appearance-dependent UI only after that has happened.
    window.show()?;
    app_controller::initialize(&window);
    slint::run_event_loop()?;
    window.hide()
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: slint::android::AndroidApp) {
    if let Err(error) = android_folder_picker::initialize(app.clone()) {
        eprintln!("Android folder picker initialization failed: {error}");
        return;
    }
    if let Err(error) = android_foreground_execution::initialize(app.clone()) {
        eprintln!("Android foreground execution initialization failed: {error}");
        return;
    }
    if let Err(error) = android_network_access::initialize(app.clone()) {
        eprintln!("Android network access check initialization failed: {error}");
        return;
    }
    if let Err(error) = slint::android::init(app) {
        eprintln!("Android UI backend initialization failed: {error}");
        return;
    }
    #[cfg(feature = "feasibility-probes")]
    {
        android_foreground_execution::schedule_probe();
        android_folder_picker::schedule_probe();
    }
    if let Err(error) = run() {
        eprintln!("SyncPak UI failed: {error}");
    }
}
