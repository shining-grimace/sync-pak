use super::{credential_deadline::check, credential_unlock::unlock};
use crate::capabilities::{CapabilityError, CredentialInteraction, CredentialPhase};
use secret_service::SecretService;
use std::collections::HashMap;
const SERVICE_NAME: &str = "com.shininggrimace.syncpak.providers";

pub(super) enum Operation<'a> {
    Load,
    Save(&'a [u8]),
    Delete,
}

fn attributes(provider_id: &str) -> HashMap<&str, &str> {
    HashMap::from([("service", SERVICE_NAME), ("username", provider_id)])
}

pub(super) async fn execute(
    service: &SecretService<'_>,
    connection: &zbus::Connection,
    destination: &str,
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
            unlock(connection, destination, item.item_path.clone(), interaction).await?;
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
        unlock(
            connection,
            destination,
            collection.collection_path.clone(),
            interaction,
        )
        .await?;
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
