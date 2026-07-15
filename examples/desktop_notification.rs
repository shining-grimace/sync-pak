use sync_pak::capabilities::{DesktopNotification, DesktopNotifier};
use sync_pak::notifications::PlatformDesktopNotifier;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let notifier = platform_notifier();
    notifier.show(&DesktopNotification {
        title: "SyncPak notification check",
        body: "Desktop notifications are available on this device.",
    })?;
    Ok(())
}

#[cfg(not(target_os = "windows"))]
fn platform_notifier() -> PlatformDesktopNotifier {
    PlatformDesktopNotifier::new()
}

#[cfg(target_os = "windows")]
fn platform_notifier() -> PlatformDesktopNotifier {
    std::env::var("SYNCPAK_WINDOWS_APP_ID").map_or_else(
        |_| PlatformDesktopNotifier::new(),
        |app_id| PlatformDesktopNotifier::new().with_windows_app_id(app_id),
    )
}
