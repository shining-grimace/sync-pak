use crate::capabilities::{CapabilityError, DesktopNotification, DesktopNotifier};

#[cfg(any(target_os = "linux", target_os = "windows"))]
const APPLICATION_NAME: &str = "SyncPak";

#[derive(Default)]
pub struct PlatformDesktopNotifier {
    #[cfg(target_os = "windows")]
    windows_app_id: Option<String>,
}

impl PlatformDesktopNotifier {
    pub fn new() -> Self {
        Self::default()
    }

    #[cfg(target_os = "windows")]
    pub fn with_windows_app_id(mut self, app_id: impl Into<String>) -> Self {
        self.windows_app_id = Some(app_id.into());
        self
    }
}

impl DesktopNotifier for PlatformDesktopNotifier {
    fn show(&self, notification: &DesktopNotification<'_>) -> Result<(), CapabilityError> {
        show_native(self, notification)
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn show_native(
    _notifier: &PlatformDesktopNotifier,
    notification: &DesktopNotification<'_>,
) -> Result<(), CapabilityError> {
    let mut native_notification = notify_rust::Notification::new();
    native_notification
        .appname(APPLICATION_NAME)
        .summary(notification.title)
        .body(notification.body);

    #[cfg(target_os = "windows")]
    if let Some(app_id) = &_notifier.windows_app_id {
        native_notification.app_id(app_id);
    }

    native_notification
        .show()
        .map(|_| ())
        .map_err(|_| CapabilityError::Unavailable)
}

#[cfg(target_os = "android")]
fn show_native(
    _notifier: &PlatformDesktopNotifier,
    _notification: &DesktopNotification<'_>,
) -> Result<(), CapabilityError> {
    Err(CapabilityError::Unsupported)
}
