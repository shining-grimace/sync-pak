use std::sync::Arc;

use keyring_core::{CredentialStore, Entry};

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
    crate::android_folder_picker::pick_folder(completion)
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
