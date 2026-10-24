use std::cell::Cell;

use super::*;

#[derive(Default)]
struct TimeoutOnce {
    inner: MemoryCredentials,
    failure: Cell<Option<CapabilityError>>,
}

impl ProtectedCredentialStore for TimeoutOnce {
    fn load(&self, id: &str) -> Result<Vec<u8>, CapabilityError> {
        self.inner.load(id)
    }

    fn save(&self, id: &str, secret: &[u8]) -> Result<(), CapabilityError> {
        if let Some(error) = self.failure.replace(None) {
            return Err(error);
        }
        self.inner.save(id, secret)
    }

    fn delete(&self, id: &str) -> Result<(), CapabilityError> {
        self.inner.delete(id)
    }
}

#[test]
fn cancelled_or_timed_out_unlocks_do_not_publish_metadata_and_can_be_retried() {
    for failure in [CapabilityError::TimedOut, CapabilityError::Cancelled] {
        let store = ConfigStore::at(test_path());
        let secrets = TimeoutOnce::default();
        secrets.failure.set(Some(failure));
        let providers = ProviderRepository::new(&store, &secrets);

        assert!(matches!(
            providers.create(provider_draft(), &secret()),
            Err(super::super::CredentialError::ProtectedStore(
                error
            )) if error == failure
        ));
        assert!(store.load().unwrap().providers.is_empty());
        assert!(secrets.inner.is_empty());

        let provider = providers.create(provider_draft(), &secret()).unwrap();
        let original_secret = secrets.load(provider.id.as_str()).unwrap();
        secrets.failure.set(Some(failure));
        assert!(matches!(
            providers.update(&provider.id, provider_draft(), &replacement_secret()),
            Err(super::super::CredentialError::ProtectedStore(
                error
            )) if error == failure
        ));
        assert_eq!(secrets.load(provider.id.as_str()), Ok(original_secret));
        assert_eq!(store.load().unwrap().providers, vec![provider.clone()]);

        // The failed update released its edit reservation, so retrying this ID works.
        providers
            .update(&provider.id, provider_draft(), &replacement_secret())
            .unwrap();
        assert!(providers.load_credentials(&provider.id).unwrap() == replacement_secret());
    }
}
