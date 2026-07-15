pub mod capabilities;
#[cfg(test)]
mod feasibility;
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
    slint::android::init(app).expect("the Android backend should initialize");
    run().expect("the SyncPak UI should run");
}
