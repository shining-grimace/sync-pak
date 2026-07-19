pub mod activity_snapshot;
pub mod add_only_execution;
#[cfg(target_os = "android")]
mod android_folder_picker;
#[cfg(target_os = "android")]
mod android_foreground_execution;
mod app_controller;
pub mod atomic_write;
pub mod cancellation;
pub mod capabilities;
pub mod comparison;
pub mod configuration;
mod connection_delete_controller;
mod connection_form_controller;
mod connection_form_data;
mod connection_form_state;
mod connection_list_controller;
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
pub mod local_inventory;
pub mod local_remote_transfer;
pub mod multipart_file_upload;
pub mod multipart_upload;
pub mod notifications;
mod onboarding;
pub mod plan_summary;
pub mod planning;
pub mod platform;
pub mod preflight;
pub mod preflight_review;
pub mod provider_capabilities;
pub mod provider_conformance;
mod provider_delete_controller;
mod provider_form;
mod provider_form_controller;
mod provider_list_controller;
pub mod provider_multipart_conformance;
#[cfg(feature = "provider-probes")]
pub mod provider_probe;
#[cfg(feature = "provider-probes")]
mod provider_probe_config;
pub mod queue;
pub mod queue_runner;
pub mod remote_inventory;
pub mod retry;
#[cfg(feature = "provider-s3")]
mod s3_error;
#[cfg(feature = "provider-s3")]
mod s3_multipart;
#[cfg(feature = "provider-s3")]
mod s3_settings;
#[cfg(feature = "provider-s3")]
pub mod s3_transport;
#[cfg(feature = "provider-s3")]
mod s3_writer;
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
    app_controller::initialize(&window);
    window.run()
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: slint::android::AndroidApp) {
    android_folder_picker::initialize(app.clone())
        .expect("the Android folder picker should initialize");
    android_foreground_execution::initialize(app.clone())
        .expect("Android foreground execution should initialize");
    slint::android::init(app).expect("the Android backend should initialize");
    #[cfg(feature = "feasibility-probes")]
    {
        android_foreground_execution::schedule_probe();
        android_folder_picker::schedule_probe();
    }
    run().expect("the SyncPak UI should run");
}
