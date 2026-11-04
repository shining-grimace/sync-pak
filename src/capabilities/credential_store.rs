use super::{CapabilityError, CredentialBackend};

/// Stores provider credentials using protection supplied by the host platform.
pub trait ProtectedCredentialStore {
    fn default_backend(&self) -> Option<CredentialBackend> {
        Some(CredentialBackend::System)
    }
    fn save(&self, provider_id: &str, credential_json: &[u8]) -> Result<(), CapabilityError>;
    fn load(&self, provider_id: &str) -> Result<Vec<u8>, CapabilityError>;
    fn delete(&self, provider_id: &str) -> Result<(), CapabilityError>;
    fn save_to(
        &self,
        _: CredentialBackend,
        id: &str,
        secret: &[u8],
    ) -> Result<(), CapabilityError> {
        self.save(id, secret)
    }
    fn load_from(&self, _: CredentialBackend, id: &str) -> Result<Vec<u8>, CapabilityError> {
        self.load(id)
    }
    fn delete_from(&self, _: CredentialBackend, id: &str) -> Result<(), CapabilityError> {
        self.delete(id)
    }
}
