#[cfg(target_os = "android")]
mod android_folder_picker;
#[cfg(target_os = "android")]
mod android_foreground_execution;
mod app_controller;
pub mod capabilities;
pub mod configuration;
mod connection_controller;
mod connection_delete_controller;
#[cfg(test)]
mod feasibility;
mod folder_picker_controller;
mod form_validation;
pub mod notifications;
pub mod platform;
mod provider_delete_controller;
mod provider_form;
#[cfg(feature = "provider-probes")]
pub mod provider_probe;
#[cfg(feature = "provider-probes")]
mod provider_probe_config;

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
