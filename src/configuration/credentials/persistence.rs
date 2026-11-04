use super::{CredentialError, ProviderCredentials, ProviderRepository};
use crate::{
    capabilities::{CapabilityError, ProtectedCredentialStore},
    configuration::{ConfigurationError, ProviderConfig},
};
impl<S: ProtectedCredentialStore> ProviderRepository<'_, S> {
    pub(super) fn save(
        &self,
        provider: ProviderConfig,
        secret: &ProviderCredentials,
    ) -> Result<(), CredentialError> {
        let _edit =
            super::super::run_guard::edit_provider(self.configuration.path(), provider.id.as_str())
                .map_err(CredentialError::Configuration)?;
        let provider_id = provider.id.clone();
        let backend = provider.credential_reference.backend;
        let serialized = serde_json::to_vec(secret).map_err(|_| CredentialError::Serialization)?;
        let previous_secret = match self.credentials.load_from(backend, provider.id.as_str()) {
            Ok(value) => Some(value),
            Err(CapabilityError::NotFound) => None,
            Err(error) => return Err(CredentialError::ProtectedStore(error)),
        };
        self.credentials
            .save_to(backend, provider_id.as_str(), &serialized)
            .map_err(CredentialError::ProtectedStore)?;

        let mut config = match self.configuration.load() {
            Ok(config) => config,
            Err(error) => return self.restore(backend, &provider_id, previous_secret, error),
        };
        if let Some(previous) = config
            .providers
            .iter()
            .find(|item| item.id == provider.id)
            .cloned()
        {
            super::super::providers::invalidate_changed_connections(
                &mut config,
                &previous,
                &provider,
                previous_secret.as_deref() != Some(serialized.as_slice()),
            );
        }
        match config
            .providers
            .iter_mut()
            .find(|item| item.id == provider.id)
        {
            Some(existing) => *existing = provider,
            None => config.providers.push(provider),
        }
        if let Err(error) = self.configuration.save(&config) {
            return self.restore(backend, &provider_id, previous_secret, error);
        }
        Ok(())
    }

    fn restore(
        &self,
        backend: crate::capabilities::CredentialBackend,
        provider_id: &crate::configuration::ProviderId,
        previous_secret: Option<Vec<u8>>,
        original_error: ConfigurationError,
    ) -> Result<(), CredentialError> {
        match previous_secret {
            Some(secret) => self
                .credentials
                .save_to(backend, provider_id.as_str(), &secret),
            None => self.credentials.delete_from(backend, provider_id.as_str()),
        }
        .map_err(|_| CredentialError::RollbackFailed)
        .map(|_| ())?;
        Err(CredentialError::Configuration(original_error))
    }
}
