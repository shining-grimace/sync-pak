#[cfg(target_os = "android")]
mod android_folder_picker;
pub mod capabilities;
#[cfg(test)]
mod feasibility;
pub mod notifications;
pub mod platform;

pub use capabilities::CapabilityError;

slint::include_modules!();

/// Opens the SyncPak application window and runs its event loop.
pub fn run() -> Result<(), slint::PlatformError> {
    AppWindow::new()?.run()
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: slint::android::AndroidApp) {
    android_folder_picker::initialize(app.clone())
        .expect("the Android folder picker should initialize");
    slint::android::init(app).expect("the Android backend should initialize");
    #[cfg(feature = "feasibility-probes")]
    android_folder_picker::schedule_probe();
    run().expect("the SyncPak UI should run");
}
