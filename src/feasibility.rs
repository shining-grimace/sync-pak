use crate::capabilities::ProtectedCredentialStore;

const PROBE_PROVIDER_ID: &str = "00000000-0000-0000-0000-feasibility";
const PROBE_CREDENTIAL: &[u8] = br#"{"probe":true}"#;

fn round_trip(store: &impl ProtectedCredentialStore) -> Result<(), crate::CapabilityError> {
    store.save(PROBE_PROVIDER_ID, PROBE_CREDENTIAL)?;
    let loaded = store.load(PROBE_PROVIDER_ID);
    let deleted = store.delete(PROBE_PROVIDER_ID);

    if loaded? != PROBE_CREDENTIAL {
        return Err(crate::CapabilityError::Unexpected);
    }
    deleted
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;
    use std::sync::atomic::{AtomicBool, Ordering};

    use super::*;

    #[derive(Default)]
    struct MemoryStore(Mutex<Option<Vec<u8>>>);

    #[derive(Default)]
    struct FailingLoadStore(AtomicBool);

    impl ProtectedCredentialStore for MemoryStore {
        fn save(&self, _: &str, value: &[u8]) -> Result<(), crate::CapabilityError> {
            *self.0.lock().unwrap() = Some(value.to_vec());
            Ok(())
        }

        fn load(&self, _: &str) -> Result<Vec<u8>, crate::CapabilityError> {
            self.0
                .lock()
                .unwrap()
                .clone()
                .ok_or(crate::CapabilityError::NotFound)
        }

        fn delete(&self, _: &str) -> Result<(), crate::CapabilityError> {
            self.0.lock().unwrap().take();
            Ok(())
        }
    }

    impl ProtectedCredentialStore for FailingLoadStore {
        fn save(&self, _: &str, _: &[u8]) -> Result<(), crate::CapabilityError> {
            Ok(())
        }

        fn load(&self, _: &str) -> Result<Vec<u8>, crate::CapabilityError> {
            Err(crate::CapabilityError::Unexpected)
        }

        fn delete(&self, _: &str) -> Result<(), crate::CapabilityError> {
            self.0.store(true, Ordering::Relaxed);
            Ok(())
        }
    }

    #[test]
    fn credential_probe_removes_the_test_value() {
        let store = MemoryStore::default();
        round_trip(&store).unwrap();
        assert_eq!(
            store.load(PROBE_PROVIDER_ID),
            Err(crate::CapabilityError::NotFound)
        );
    }

    #[test]
    fn credential_probe_cleans_up_after_a_read_failure() {
        let store = FailingLoadStore::default();
        assert_eq!(round_trip(&store), Err(crate::CapabilityError::Unexpected));
        assert!(store.0.load(Ordering::Relaxed));
    }
}
