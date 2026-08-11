use std::sync::Arc;

#[cfg(target_os = "android")]
pub(crate) mod android;
pub mod atomic_write;
#[cfg(test)]
pub(crate) mod feasibility;
pub mod notifications;
pub mod temporary_cleanup;

use keyring_core::{CredentialStore, Entry};

#[cfg(target_os = "android")]
use crate::capabilities::BackgroundExecution;
use crate::capabilities::{
    CapabilityError, FolderPicker, FolderPickerCompletion, ProtectedCredentialStore,
};

const SERVICE_NAME: &str = "com.shininggrimace.syncpak.providers";

pub struct PlatformCredentialStore {
    store: Arc<CredentialStore>,
}

impl PlatformCredentialStore {
    pub fn new() -> Result<Self, CapabilityError> {
        platform_credential_store()
            .map(|store| Self { store })
            .map_err(map_keyring_error)
    }

    fn entry(&self, provider_id: &str) -> Result<Entry, CapabilityError> {
        self.store
            .build(SERVICE_NAME, provider_id, None)
            .map_err(map_keyring_error)
    }
}

impl ProtectedCredentialStore for PlatformCredentialStore {
    fn save(&self, provider_id: &str, credential_json: &[u8]) -> Result<(), CapabilityError> {
        self.entry(provider_id)?
            .set_secret(credential_json)
            .map_err(map_keyring_error)
    }

    fn load(&self, provider_id: &str) -> Result<Vec<u8>, CapabilityError> {
        self.entry(provider_id)?
            .get_secret()
            .map_err(map_keyring_error)
    }

    fn delete(&self, provider_id: &str) -> Result<(), CapabilityError> {
        self.entry(provider_id)?
            .delete_credential()
            .map_err(map_keyring_error)
    }
}

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
}

#[cfg(target_os = "linux")]
fn platform_credential_store() -> keyring_core::Result<Arc<CredentialStore>> {
    zbus_secret_service_keyring_store::Store::new().map(|store| store as Arc<CredentialStore>)
}

#[cfg(target_os = "windows")]
fn platform_credential_store() -> keyring_core::Result<Arc<CredentialStore>> {
    windows_native_keyring_store::Store::new().map(|store| store as Arc<CredentialStore>)
}

#[cfg(target_os = "android")]
fn platform_credential_store() -> keyring_core::Result<Arc<CredentialStore>> {
    android_native_keyring_store::Store::new().map(|store| store as Arc<CredentialStore>)
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

fn map_keyring_error(error: keyring_core::Error) -> CapabilityError {
    match error {
        keyring_core::Error::NoEntry => CapabilityError::NotFound,
        keyring_core::Error::Invalid(_, _) | keyring_core::Error::TooLong(_, _) => {
            CapabilityError::InvalidReference
        }
        keyring_core::Error::NoStorageAccess(_) | keyring_core::Error::NoDefaultStore => {
            CapabilityError::Unavailable
        }
        keyring_core::Error::NotSupportedByStore(_) => CapabilityError::Unsupported,
        _ => CapabilityError::Unexpected,
    }
}
