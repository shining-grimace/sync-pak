use super::*;
use crate::capabilities::CredentialBackend;

#[derive(Default)]
struct Stores(Mutex<HashMap<(CredentialBackend, String), Vec<u8>>>);

impl ProtectedCredentialStore for Stores {
    fn default_backend(&self) -> Option<CredentialBackend> {
        None
    }
    fn load(&self, _: &str) -> Result<Vec<u8>, CapabilityError> {
        Err(CapabilityError::Unexpected)
    }
    fn save(&self, _: &str, _: &[u8]) -> Result<(), CapabilityError> {
        Err(CapabilityError::Unexpected)
    }
    fn delete(&self, _: &str) -> Result<(), CapabilityError> {
        Err(CapabilityError::Unexpected)
    }
    fn load_from(&self, backend: CredentialBackend, id: &str) -> Result<Vec<u8>, CapabilityError> {
        self.0
            .lock()
            .unwrap()
            .get(&(backend, id.into()))
            .cloned()
            .ok_or(CapabilityError::NotFound)
    }
    fn save_to(
        &self,
        backend: CredentialBackend,
        id: &str,
        secret: &[u8],
    ) -> Result<(), CapabilityError> {
        self.0
            .lock()
            .unwrap()
            .insert((backend, id.into()), secret.to_vec());
        Ok(())
    }
    fn delete_from(&self, backend: CredentialBackend, id: &str) -> Result<(), CapabilityError> {
        self.0
            .lock()
            .unwrap()
            .remove(&(backend, id.into()))
            .ok_or(CapabilityError::NotFound)
            .map(|_| ())
    }
}

#[test]
fn changing_the_preference_only_affects_new_cloud_providers() {
    let config = ConfigStore::at(test_path());
    let stores = Stores::default();
    let repository = ProviderRepository::new(&config, &stores);
    assert!(matches!(
        repository.create(provider_draft(), &secret()),
        Err(super::super::CredentialError::ProtectedStore(
            CapabilityError::NotConfigured
        ))
    ));
    let mut settings = config.load().unwrap();
    settings.credential_backend = Some(CredentialBackend::Kde);
    config.save(&settings).unwrap();
    let original = repository.create(provider_draft(), &secret()).unwrap();
    settings = config.load().unwrap();
    settings.credential_backend = Some(CredentialBackend::Gnome);
    config.save(&settings).unwrap();
    assert!(repository.load_credentials(&original.id).unwrap() == secret());
    let edited = repository
        .update(&original.id, provider_draft(), &replacement_secret())
        .unwrap();
    assert_eq!(edited.credential_reference.backend, CredentialBackend::Kde);
    let new_provider = repository.create(provider_draft(), &secret()).unwrap();
    assert_eq!(
        new_provider.credential_reference.backend,
        CredentialBackend::Gnome
    );
    assert!(repository.load_credentials(&original.id).unwrap() == replacement_secret());
    repository.delete(&original.id).unwrap();
    assert!(
        stores
            .load_from(CredentialBackend::Kde, original.id.as_str())
            .is_err()
    );
    assert!(repository.load_credentials(&new_provider.id).unwrap() == secret());
    assert_eq!(config.load().unwrap().schema_version, 1);
}
