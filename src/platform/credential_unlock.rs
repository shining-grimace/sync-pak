//! Keep D-Bus response deadlines separate from time spent answering a desktop prompt.
use futures_lite::StreamExt;
use std::time::Duration;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};

use super::{
    credential_deadline::{SERVICE_TIMEOUT, UNLOCK_TIMEOUT, controlled, within},
    credential_errors::map_bus_error,
};
use crate::capabilities::{CapabilityError, CredentialInteraction, CredentialPhase};

#[zbus::proxy(
    interface = "org.freedesktop.Secret.Service",
    default_service = "org.freedesktop.secrets",
    default_path = "/org/freedesktop/secrets"
)]
trait UnlockService {
    fn unlock(
        &self,
        objects: Vec<OwnedObjectPath>,
    ) -> zbus::Result<(Vec<OwnedObjectPath>, OwnedObjectPath)>;
}

#[zbus::proxy(
    interface = "org.freedesktop.Secret.Prompt",
    default_service = "org.freedesktop.secrets"
)]
trait UnlockPrompt {
    fn prompt(&self, window_id: &str) -> zbus::Result<()>;
    fn dismiss(&self) -> zbus::Result<()>;
    #[zbus(signal)]
    fn completed(&self, dismissed: bool, result: OwnedValue) -> zbus::Result<()>;
}

pub(super) async fn unlock(
    connection: &zbus::Connection,
    path: OwnedObjectPath,
    interaction: &CredentialInteraction,
) -> Result<(), CapabilityError> {
    let prompt_path = controlled(interaction, SERVICE_TIMEOUT, async {
        let service = UnlockServiceProxy::new(connection)
            .await
            .map_err(map_bus_error)?;
        let (_, prompt) = service.unlock(vec![path]).await.map_err(map_bus_error)?;
        Ok(prompt)
    })
    .await?;
    if prompt_path.as_str() == "/" {
        return Ok(());
    }
    interaction.transition(CredentialPhase::Unlocking)?;
    // Subscribe before requesting the prompt so immediate completion is not lost.
    let (prompt, mut completed) = controlled(interaction, SERVICE_TIMEOUT, async {
        let prompt = UnlockPromptProxy::builder(connection)
            .path(prompt_path)
            .map_err(map_bus_error)?
            .build()
            .await
            .map_err(map_bus_error)?;
        let completed = prompt.receive_completed().await.map_err(map_bus_error)?;
        prompt.prompt("").await.map_err(map_bus_error)?;
        Ok((prompt, completed))
    })
    .await?;
    let result = controlled(interaction, UNLOCK_TIMEOUT, async {
        let signal = completed.next().await.ok_or(CapabilityError::Unavailable)?;
        let args = signal.args().map_err(map_bus_error)?;
        if args.dismissed {
            Err(CapabilityError::Cancelled)
        } else {
            Ok(())
        }
    })
    .await;
    if result.is_err() {
        // Cancellation must dismiss an actual password dialog as well as stop our wait.
        // Bound cleanup independently, since the interaction may already be cancelled.
        let _ = within(Duration::from_secs(1), async {
            prompt.dismiss().await.map_err(map_bus_error)
        })
        .await;
    }
    // The caller also closes the dedicated connection, invalidating any remaining prompt.
    result.map_err(|error| {
        if error == CapabilityError::TimedOut {
            CapabilityError::UnlockTimedOut
        } else {
            error
        }
    })
}
