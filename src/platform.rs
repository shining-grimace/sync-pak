#[cfg(target_os = "linux")]
mod credential_deadline;
#[cfg(target_os = "linux")]
mod credential_errors;
#[cfg(target_os = "linux")]
mod credential_unlock;
#[cfg(target_os = "linux")]
mod linux_credentials;
#[cfg(target_os = "linux")]
pub use linux_credentials::PlatformCredentialStore;
#[cfg(not(target_os = "linux"))]
mod credentials;
#[cfg(not(target_os = "linux"))]
pub use credentials::PlatformCredentialStore;

#[cfg(target_os = "android")]
pub(crate) mod android;
pub mod atomic_write;
pub mod list_file;
pub(crate) mod advertising {
    #[cfg(target_os = "android")]
    pub fn set_placement(placement: i32) {
        super::android::advertising::set_placement(placement);
    }

    #[cfg(not(target_os = "android"))]
    pub fn set_placement(_: i32) {}

    #[cfg(target_os = "android")]
    pub fn request_consent() {
        super::android::advertising::request_consent();
    }

    #[cfg(not(target_os = "android"))]
    pub fn request_consent() {}

    #[cfg(target_os = "android")]
    pub fn show_privacy_choices() {
        super::android::advertising::show_privacy_choices();
    }

    #[cfg(not(target_os = "android"))]
    pub fn show_privacy_choices() {}
}
#[cfg(test)]
pub(crate) mod feasibility;
pub mod notifications;
pub mod temporary_cleanup;

#[cfg(target_os = "android")]
use crate::capabilities::BackgroundExecution;
use crate::capabilities::{CapabilityError, FolderPicker, FolderPickerCompletion};

pub struct PlatformFolderPicker;

impl FolderPicker for PlatformFolderPicker {
    fn pick_folder(&self, completion: FolderPickerCompletion) -> Result<(), CapabilityError> {
        pick_folder(completion)
    }
}

#[cfg(target_os = "android")]
pub struct PlatformBackgroundExecution;

#[cfg(target_os = "android")]
impl BackgroundExecution for PlatformBackgroundExecution {
    fn start(&self, connection_name: &str) -> Result<(), CapabilityError> {
        start_background_execution(connection_name)
    }

    fn update(
        &self,
        connection_name: &str,
        progress: &crate::operations::operation_progress::OperationProgress,
    ) -> Result<(), CapabilityError> {
        update_background_execution(connection_name, progress)
    }

    fn stop(&self) -> Result<(), CapabilityError> {
        stop_background_execution()
    }
    fn finish_operation(
        &self,
        connection_name: &str,
        summary: &str,
        operation_id: &str,
    ) -> Result<(), CapabilityError> {
        crate::platform::android::foreground_execution::finish_operation(
            connection_name,
            summary,
            operation_id,
        )
    }
    fn finish(&self, connection_name: &str, summary: &str) -> Result<(), CapabilityError> {
        crate::platform::android::foreground_execution::finish(connection_name, summary)
    }
}

#[cfg(any(target_os = "linux", target_os = "windows"))]
fn pick_folder(completion: FolderPickerCompletion) -> Result<(), CapabilityError> {
    let selection = rfd::FileDialog::new()
        .set_title("Choose a folder for SyncPak")
        .pick_folder()
        .map(crate::capabilities::FolderSelection::FileSystem);
    completion(Ok(selection));
    Ok(())
}

#[cfg(target_os = "android")]
fn pick_folder(completion: FolderPickerCompletion) -> Result<(), CapabilityError> {
    crate::platform::android::folder_picker::pick_folder(completion)
}

#[cfg(target_os = "android")]
fn start_background_execution(connection_name: &str) -> Result<(), CapabilityError> {
    crate::platform::android::foreground_execution::start(connection_name)
}

#[cfg(target_os = "android")]
fn update_background_execution(
    connection_name: &str,
    progress: &crate::operations::operation_progress::OperationProgress,
) -> Result<(), CapabilityError> {
    crate::platform::android::foreground_execution::update(connection_name, progress)
}

#[cfg(target_os = "android")]
fn stop_background_execution() -> Result<(), CapabilityError> {
    crate::platform::android::foreground_execution::stop()
}
