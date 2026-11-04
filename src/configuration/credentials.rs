use serde::{Deserialize, Serialize};

use crate::capabilities::{CapabilityError, ProtectedCredentialStore};

use super::{ConfigStore, ConfigurationError, CredentialReference, ProviderConfig, ProviderId};
use super::{DeletedProvider, ProviderDraft};

/// Secret values for a provider. This type is never part of [`super::AppConfig`].
#[derive(Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderCredentials {
    pub access_key_id: String,
    pub secret_access_key: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session_token: Option<String>,
}

#[derive(Debug)]
pub enum CredentialError {
    Configuration(ConfigurationError),
    ProtectedStore(CapabilityError),
    Serialization,
    RollbackFailed,
    NotFound,
}

impl std::fmt::Display for CredentialError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(error) => {
                write!(formatter, "Provider settings were not saved: {error}")
            }
            Self::ProtectedStore(error) => write!(
                formatter,
                "Protected credential storage is unavailable: {error}"
            ),
            Self::Serialization => formatter
                .write_str("Provider credentials could not be prepared for secure storage."),
            Self::RollbackFailed => formatter.write_str(
                "Provider settings were not saved and protected credential recovery failed.",
            ),
            Self::NotFound => formatter.write_str("The provider no longer exists."),
        }
    }
}

impl std::error::Error for CredentialError {}

pub struct ProviderRepository<'a, S> {
    configuration: &'a ConfigStore,
    credentials: &'a S,
}

impl<'a, S: ProtectedCredentialStore> ProviderRepository<'a, S> {
    pub fn new(configuration: &'a ConfigStore, credentials: &'a S) -> Self {
        Self {
            configuration,
            credentials,
        }
    }

    /// Loads credentials only for a provider still present in configuration.
    pub fn load_credentials(
        &self,
        id: &ProviderId,
    ) -> Result<ProviderCredentials, CredentialError> {
        let config = self
            .configuration
            .load()
            .map_err(CredentialError::Configuration)?;
        let backend = config
            .providers
            .iter()
            .find(|provider| provider.id == *id)
            .ok_or(CredentialError::NotFound)?
            .credential_reference
            .backend;
        let serialized = self
            .credentials
            .load_from(backend, id.as_str())
            .map_err(CredentialError::ProtectedStore)?;
        serde_json::from_slice(&serialized).map_err(|_| CredentialError::Serialization)
    }

    /// Saves the protected document before committing metadata, restoring it if that commit fails.
    pub fn create(
        &self,
        draft: ProviderDraft,
        secret: &ProviderCredentials,
    ) -> Result<ProviderConfig, CredentialError> {
        let id = ProviderId::new();
        let provider = ProviderConfig {
            credential_reference: CredentialReference {
                backend: self
                    .configuration
                    .load()
                    .map_err(CredentialError::Configuration)?
                    .credential_backend
                    .or_else(|| self.credentials.default_backend())
                    .ok_or(CredentialError::ProtectedStore(
                        CapabilityError::NotConfigured,
                    ))?,
                provider_id: id.clone(),
            },
            id,
            name: draft.name,
            kind: draft.kind,
            options: draft.options,
            verified: draft.verified,
        };
        self.save(provider.clone(), secret)?;
        Ok(provider)
    }

    pub fn update(
        &self,
        id: &ProviderId,
        draft: ProviderDraft,
        secret: &ProviderCredentials,
    ) -> Result<ProviderConfig, CredentialError> {
        let provider = ProviderConfig {
            id: id.clone(),
            credential_reference: CredentialReference {
                backend: self
                    .configuration
                    .load()
                    .map_err(CredentialError::Configuration)?
                    .providers
                    .iter()
                    .find(|provider| provider.id == *id)
                    .ok_or(CredentialError::NotFound)?
                    .credential_reference
                    .backend,
                provider_id: id.clone(),
            },
            name: draft.name,
            kind: draft.kind,
            options: draft.options,
            verified: draft.verified,
        };
        let config = self
            .configuration
            .load()
            .map_err(CredentialError::Configuration)?;
        if !config.providers.iter().any(|provider| provider.id == *id) {
            return Err(CredentialError::NotFound);
        }
        self.save(provider.clone(), secret)?;
        Ok(provider)
    }

    pub fn delete(&self, id: &ProviderId) -> Result<DeletedProvider, CredentialError> {
        let _edit = super::run_guard::edit_provider(self.configuration.path(), id.as_str())
            .map_err(CredentialError::Configuration)?;
        let mut config = self
            .configuration
            .load()
            .map_err(CredentialError::Configuration)?;
        let index = config
            .providers
            .iter()
            .position(|provider| provider.id == *id)
            .ok_or(CredentialError::NotFound)?;
        let provider = config.providers.remove(index);
        let dependent_connection_ids = config
            .connections
            .iter()
            .filter(|connection| connection.provider_id == *id)
            .map(|connection| connection.id.clone())
            .collect();
        config
            .connections
            .retain(|connection| connection.provider_id != *id);

        let backend = provider.credential_reference.backend;
        let secret = self
            .credentials
            .load_from(backend, id.as_str())
            .map_err(CredentialError::ProtectedStore)?;
        self.credentials
            .delete_from(backend, id.as_str())
            .map_err(CredentialError::ProtectedStore)?;
        if let Err(error) = self.configuration.save(&config) {
            self.credentials
                .save_to(backend, id.as_str(), &secret)
                .map_err(|_| CredentialError::RollbackFailed)?;
            return Err(CredentialError::Configuration(error));
        }
        Ok(DeletedProvider {
            provider,
            dependent_connection_ids,
        })
    }
}

mod persistence;

#[cfg(test)]
mod tests;
