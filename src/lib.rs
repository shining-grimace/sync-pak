mod app;
pub mod capabilities;
mod configuration;
mod inventory;
mod operations;
mod platform;
mod preflight;
mod providers;
mod sync_cache;
mod validation;

pub use capabilities::CapabilityError;
pub use platform::notifications;
#[cfg(feature = "provider-probes")]
pub use providers::probe as provider_probe;

mod ui {
    slint::include_modules!();
}

pub(crate) use ui::*;

/// Opens the SyncPak application window and runs its event loop.
pub fn run() -> Result<(), slint::PlatformError> {
    let window = AppWindow::new()?;
    #[cfg(target_os = "linux")]
    {
        // Snap prefixes exported desktop filenames, including parallel-install instances.
        let app_id = std::env::var("SNAP_INSTANCE_NAME")
            .map(|instance| format!("{instance}_sync-pak"))
            .unwrap_or_else(|_| "com.shininggrimace.SyncPak".into());
        slint::set_xdg_app_id(app_id)?;
    }
    // The desktop backend resolves the native colour scheme when the window is
    // shown. Initialise appearance-dependent UI only after that has happened.
    window.show()?;
    app::controller::initialize(&window);
    #[cfg(target_os = "android")]
    platform::android::advertising::configure(&window);
    slint::run_event_loop()?;
    window.hide()
}

#[cfg(target_os = "android")]
#[unsafe(no_mangle)]
pub fn android_main(app: slint::android::AndroidApp) {
    if let Err(error) = platform::android::advertising::initialize(app.clone()) {
        eprintln!("Android advertising initialization failed: {error}");
        return;
    }
    if let Err(error) = platform::android::folder_picker::initialize(app.clone()) {
        eprintln!("Android folder picker initialization failed: {error}");
        return;
    }
    if let Err(error) = slint::android::init(app) {
        eprintln!("Android UI backend initialization failed: {error}");
        return;
    }
    #[cfg(feature = "feasibility-probes")]
    {
        platform::android::foreground_execution::schedule_probe();
        platform::android::folder_picker::schedule_probe();
    }
    if let Err(error) = run() {
        eprintln!("SyncPak UI failed: {error}");
    }
}
