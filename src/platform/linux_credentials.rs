//! Bounded Secret Service access, preserving the keyring wrapper's item identity.
use std::time::Duration;

use secret_service::{EncryptionType, SecretService};

use super::{
    credential_deadline::{SERVICE_TIMEOUT, check, controlled, within},
    credential_operations::{Operation, execute},
};
use crate::capabilities::{
    CapabilityError, CredentialBackend, CredentialInteraction, CredentialPhase,
    ProtectedCredentialStore,
};

pub struct PlatformCredentialStore {
    interaction: CredentialInteraction,
}

impl PlatformCredentialStore {
    pub fn new() -> Result<Self, CapabilityError> {
        Self::with_interaction(CredentialInteraction::default())
    }

    pub(crate) fn with_interaction(
        interaction: CredentialInteraction,
    ) -> Result<Self, CapabilityError> {
        Ok(Self { interaction })
    }

    fn request(
        &self,
        backend: CredentialBackend,
        provider_id: &str,
        operation: Operation<'_>,
    ) -> Result<Vec<u8>, CapabilityError> {
        async_io::block_on(async {
            let interaction = &self.interaction;
            interaction.transition(CredentialPhase::Checking)?;
            let connection = controlled(interaction, SERVICE_TIMEOUT, async {
                zbus::Connection::session()
                    .await
                    .map_err(super::credential_errors::map_bus_error)
            })
            .await?;
            let result = async {
                let service = check(
                    interaction,
                    SecretService::connect_with_destination(
                        EncryptionType::Dh,
                        connection.clone(),
                        backend.destination(),
                    ),
                )
                .await?;
                execute(
                    &service,
                    &connection,
                    backend.destination(),
                    interaction,
                    provider_id,
                    operation,
                )
                .await
            }
            .await;
            // Disconnect explicitly: prompt objects belong to this connection and become
            // invalid on disconnect. No abandoned worker can later continue into a write.
            let _ = within(Duration::from_secs(1), async {
                connection
                    .close()
                    .await
                    .map_err(|_| CapabilityError::Unavailable)
            })
            .await;
            result
        })
    }
}

impl ProtectedCredentialStore for PlatformCredentialStore {
    fn default_backend(&self) -> Option<CredentialBackend> {
        None
    }
    fn load_from(&self, backend: CredentialBackend, id: &str) -> Result<Vec<u8>, CapabilityError> {
        self.request(backend, id, Operation::Load)
    }
    fn save_to(
        &self,
        backend: CredentialBackend,
        id: &str,
        secret: &[u8],
    ) -> Result<(), CapabilityError> {
        self.request(backend, id, Operation::Save(secret))
            .map(|_| ())
    }
    fn delete_from(&self, backend: CredentialBackend, id: &str) -> Result<(), CapabilityError> {
        self.request(backend, id, Operation::Delete).map(|_| ())
    }

    fn load(&self, provider_id: &str) -> Result<Vec<u8>, CapabilityError> {
        self.request(CredentialBackend::System, provider_id, Operation::Load)
    }

    fn save(&self, provider_id: &str, secret: &[u8]) -> Result<(), CapabilityError> {
        self.request(
            CredentialBackend::System,
            provider_id,
            Operation::Save(secret),
        )
        .map(|_| ())
    }

    fn delete(&self, provider_id: &str) -> Result<(), CapabilityError> {
        self.request(CredentialBackend::System, provider_id, Operation::Delete)
            .map(|_| ())
    }
}
