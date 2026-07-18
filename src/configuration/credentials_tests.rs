use std::{
    collections::HashMap,
    fs,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use crate::{
    capabilities::{CapabilityError, ProtectedCredentialStore},
    configuration::{
        ConfigStore, ConnectionDraft, ConnectionRepository, ProviderKind, ProviderOptions, SyncMode,
    },
};

use super::{ProviderCredentials, ProviderDraft, ProviderRepository};

#[derive(Default)]
struct MemoryCredentials(Mutex<HashMap<String, Vec<u8>>>);

impl ProtectedCredentialStore for MemoryCredentials {
    fn save(&self, id: &str, secret: &[u8]) -> Result<(), CapabilityError> {
        self.0
            .lock()
            .unwrap()
            .insert(id.to_owned(), secret.to_vec());
        Ok(())
    }

    fn load(&self, id: &str) -> Result<Vec<u8>, CapabilityError> {
        self.0
            .lock()
            .unwrap()
            .get(id)
            .cloned()
            .ok_or(CapabilityError::NotFound)
    }

    fn delete(&self, id: &str) -> Result<(), CapabilityError> {
        self.0
            .lock()
            .unwrap()
            .remove(id)
            .map(|_| ())
            .ok_or(CapabilityError::NotFound)
    }
}

impl MemoryCredentials {
    fn is_empty(&self) -> bool {
        self.0.lock().unwrap().is_empty()
    }
}

#[test]
fn provider_deletion_removes_dependent_connections_and_credentials() {
    let store = ConfigStore::at(test_path());
    let secrets = MemoryCredentials::default();
    let providers = ProviderRepository::new(&store, &secrets);
    let provider = providers.create(provider_draft(), &secret()).unwrap();
    let connection = ConnectionRepository::new(&store)
        .create(ConnectionDraft {
            name: "Photos".to_owned(),
            provider_id: provider.id.clone(),
            bucket: "backup".to_owned(),
            remote_path: String::new(),
            local_path: "/photos".to_owned(),
            mode: SyncMode::AddOnly,
            keep_last_archives: None,
        })
        .unwrap();

    let deleted = providers.delete(&provider.id).unwrap();

    assert_eq!(deleted.dependent_connection_ids, vec![connection.id]);
    assert!(store.load().unwrap().providers.is_empty());
    assert!(store.load().unwrap().connections.is_empty());
    assert_eq!(
        secrets.load(provider.id.as_str()),
        Err(CapabilityError::NotFound)
    );
}

#[test]
fn ordinary_configuration_never_contains_provider_credentials() {
    let path = test_path();
    let store = ConfigStore::at(path.clone());
    ProviderRepository::new(&store, &MemoryCredentials::default())
        .create(provider_draft(), &secret())
        .unwrap();

    let saved = fs::read_to_string(path).unwrap();
    assert!(!saved.contains("secret-value"));
    assert!(!saved.contains("access-value"));
}

#[test]
fn failed_metadata_commit_removes_a_newly_saved_credential() {
    let store = ConfigStore::at(test_path());
    let secrets = MemoryCredentials::default();
    let invalid_draft = ProviderDraft {
        name: String::new(),
        ..provider_draft()
    };

    assert!(
        ProviderRepository::new(&store, &secrets)
            .create(invalid_draft, &secret())
            .is_err()
    );

    assert!(secrets.is_empty());
    assert!(store.load().unwrap().providers.is_empty());
}

#[test]
fn failed_edit_restores_the_prior_credential_and_metadata() {
    let store = ConfigStore::at(test_path());
    let secrets = MemoryCredentials::default();
    let providers = ProviderRepository::new(&store, &secrets);
    let provider = providers.create(provider_draft(), &secret()).unwrap();
    let original_credential = secrets.load(provider.id.as_str()).unwrap();
    let invalid_draft = ProviderDraft {
        name: String::new(),
        ..provider_draft()
    };

    assert!(
        providers
            .update(&provider.id, invalid_draft, &replacement_secret())
            .is_err()
    );

    assert_eq!(secrets.load(provider.id.as_str()), Ok(original_credential));
    assert_eq!(store.load().unwrap().providers, vec![provider]);
}

fn provider_draft() -> ProviderDraft {
    ProviderDraft {
        name: "Test provider".to_owned(),
        kind: ProviderKind::AwsS3,
        options: ProviderOptions {
            endpoint: None,
            region: Some("ap-southeast-2".to_owned()),
        },
    }
}

fn secret() -> ProviderCredentials {
    ProviderCredentials {
        access_key_id: "access-value".to_owned(),
        secret_access_key: "secret-value".to_owned(),
        session_token: None,
    }
}

fn replacement_secret() -> ProviderCredentials {
    ProviderCredentials {
        access_key_id: "replacement-access".to_owned(),
        secret_access_key: "replacement-secret".to_owned(),
        session_token: None,
    }
}

fn test_path() -> std::path::PathBuf {
    std::env::temp_dir()
        .join(format!(
            "sync-pak-provider-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .join("config.json")
}
