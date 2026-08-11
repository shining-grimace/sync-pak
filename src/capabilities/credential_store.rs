use super::CapabilityError;

/// Stores provider credentials using protection supplied by the host platform.
pub trait ProtectedCredentialStore {
    fn save(&self, provider_id: &str, credential_json: &[u8]) -> Result<(), CapabilityError>;
    fn load(&self, provider_id: &str) -> Result<Vec<u8>, CapabilityError>;
    fn delete(&self, provider_id: &str) -> Result<(), CapabilityError>;
}
