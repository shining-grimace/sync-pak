//! Bounded Secret Service access, preserving the keyring wrapper's item identity.
use std::{collections::HashMap, time::Duration};

use secret_service::{EncryptionType, SecretService};

use super::{
    credential_deadline::{SERVICE_TIMEOUT, check, controlled, within},
    credential_unlock::unlock,
};
use crate::capabilities::{
    CapabilityError, CredentialInteraction, CredentialPhase, ProtectedCredentialStore,
};

const SERVICE_NAME: &str = "com.shininggrimace.syncpak.providers";

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
                    SecretService::connect_with_existing(EncryptionType::Dh, connection.clone()),
                )
                .await?;
                execute(&service, &connection, interaction, provider_id, operation).await
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

enum Operation<'a> {
    Load,
    Save(&'a [u8]),
    Delete,
}

impl ProtectedCredentialStore for PlatformCredentialStore {
    fn load(&self, provider_id: &str) -> Result<Vec<u8>, CapabilityError> {
        self.request(provider_id, Operation::Load)
    }

    fn save(&self, provider_id: &str, secret: &[u8]) -> Result<(), CapabilityError> {
        self.request(provider_id, Operation::Save(secret))
            .map(|_| ())
    }

    fn delete(&self, provider_id: &str) -> Result<(), CapabilityError> {
        self.request(provider_id, Operation::Delete).map(|_| ())
    }
}

fn attributes(provider_id: &str) -> HashMap<&str, &str> {
    HashMap::from([("service", SERVICE_NAME), ("username", provider_id)])
}

async fn execute(
    service: &SecretService<'_>,
    connection: &zbus::Connection,
    interaction: &CredentialInteraction,
    provider_id: &str,
    operation: Operation<'_>,
) -> Result<Vec<u8>, CapabilityError> {
    let search = check(interaction, service.search_items(attributes(provider_id))).await?;
    let mut items = search.unlocked;
    items.extend(search.locked);
    // Preserve the old wrapper's refusal to choose between ambiguous entries.
    if items.len() > 1 {
        return Err(CapabilityError::InvalidReference);
    }
    if let Some(item) = items.pop() {
        if check(interaction, item.is_locked()).await? {
            unlock(connection, item.item_path.clone(), interaction).await?;
            if check(interaction, item.is_locked()).await? {
                return Err(CapabilityError::Locked);
            }
        }
        return match operation {
            Operation::Load => check(interaction, item.get_secret()).await,
            Operation::Save(secret) => {
                interaction.transition(CredentialPhase::Writing)?;
                check(interaction, item.set_secret(secret, "text/plain"))
                    .await
                    .map(|_| Vec::new())
            }
            Operation::Delete => {
                interaction.transition(CredentialPhase::Writing)?;
                check(interaction, item.delete()).await.map(|_| Vec::new())
            }
        };
    }
    let Operation::Save(secret) = operation else {
        return Err(CapabilityError::NotFound);
    };
    let collection = check(interaction, service.get_default_collection())
        .await
        .map_err(|error| {
            if error == CapabilityError::NotFound {
                CapabilityError::NotConfigured
            } else {
                error
            }
        })?;
    if check(interaction, collection.is_locked()).await? {
        unlock(connection, collection.collection_path.clone(), interaction).await?;
        if check(interaction, collection.is_locked()).await? {
            return Err(CapabilityError::Locked);
        }
    }
    interaction.transition(CredentialPhase::Writing)?;
    check(
        interaction,
        collection.create_item(
            &format!("keyring:{provider_id}@{SERVICE_NAME}"),
            attributes(provider_id),
            secret,
            true,
            "application/octet-stream",
        ),
    )
    .await?;
    Ok(Vec::new())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn existing_keyring_wrapper_credentials_keep_the_same_identity() {
        assert_eq!(
            attributes("provider-id"),
            HashMap::from([
                ("service", "com.shininggrimace.syncpak.providers"),
                ("username", "provider-id"),
            ])
        );
    }
}
