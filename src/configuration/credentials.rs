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
    Serialization(serde_json::Error),
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
            Self::Serialization(_) => formatter
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

    /// Saves the protected document before committing metadata, restoring it if that commit fails.
    pub fn create(
        &self,
        draft: ProviderDraft,
        secret: &ProviderCredentials,
    ) -> Result<ProviderConfig, CredentialError> {
        let id = ProviderId::new();
        let provider = ProviderConfig {
            credential_reference: CredentialReference {
                provider_id: id.clone(),
            },
            id,
            name: draft.name,
            kind: draft.kind,
            options: draft.options,
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
                provider_id: id.clone(),
            },
            name: draft.name,
            kind: draft.kind,
            options: draft.options,
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

        let secret = self
            .credentials
            .load(id.as_str())
            .map_err(CredentialError::ProtectedStore)?;
        self.credentials
            .delete(id.as_str())
            .map_err(CredentialError::ProtectedStore)?;
        if let Err(error) = self.configuration.save(&config) {
            self.credentials
                .save(id.as_str(), &secret)
                .map_err(|_| CredentialError::RollbackFailed)?;
            return Err(CredentialError::Configuration(error));
        }
        Ok(DeletedProvider {
            provider,
            dependent_connection_ids,
        })
    }

    fn save(
        &self,
        provider: ProviderConfig,
        secret: &ProviderCredentials,
    ) -> Result<(), CredentialError> {
        let provider_id = provider.id.clone();
        let serialized = serde_json::to_vec(secret).map_err(CredentialError::Serialization)?;
        let previous_secret = match self.credentials.load(provider.id.as_str()) {
            Ok(value) => Some(value),
            Err(CapabilityError::NotFound) => None,
            Err(error) => return Err(CredentialError::ProtectedStore(error)),
        };
        self.credentials
            .save(provider_id.as_str(), &serialized)
            .map_err(CredentialError::ProtectedStore)?;

        let mut config = match self.configuration.load() {
            Ok(config) => config,
            Err(error) => return self.restore(&provider_id, previous_secret, error),
        };
        match config
            .providers
            .iter_mut()
            .find(|item| item.id == provider.id)
        {
            Some(existing) => *existing = provider,
            None => config.providers.push(provider),
        }
        if let Err(error) = self.configuration.save(&config) {
            return self.restore(&provider_id, previous_secret, error);
        }
        Ok(())
    }

    fn restore(
        &self,
        provider_id: &super::ProviderId,
        previous_secret: Option<Vec<u8>>,
        original_error: ConfigurationError,
    ) -> Result<(), CredentialError> {
        match previous_secret {
            Some(secret) => self.credentials.save(provider_id.as_str(), &secret),
            None => self.credentials.delete(provider_id.as_str()),
        }
        .map_err(|_| CredentialError::RollbackFailed)
        .map(|_| ())?;
        Err(CredentialError::Configuration(original_error))
    }
}

#[cfg(test)]
#[path = "credentials_tests.rs"]
mod tests;
