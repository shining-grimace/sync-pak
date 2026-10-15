use super::{
    ConfigStore, ConfigurationError, ConnectionId, ProviderConfig, ProviderKind, ProviderOptions,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderDraft {
    pub name: String,
    pub kind: ProviderKind,
    pub options: ProviderOptions,
    pub verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeletedProvider {
    pub provider: ProviderConfig,
    pub dependent_connection_ids: Vec<ConnectionId>,
}

impl ConfigStore {
    pub(crate) fn record_provider_verification(
        &self,
        provider_id: &str,
    ) -> Result<bool, ConfigurationError> {
        let mut config = self.load()?;
        let Some(provider) = config
            .providers
            .iter_mut()
            .find(|provider| provider.id.as_str() == provider_id)
        else {
            return Ok(false);
        };
        provider.verified = true;
        self.save(&config)?;
        Ok(true)
    }
}

/// A successful check belongs to its credentials and resolved destination.
/// Changing a provider must not leave a connection verified for an old route.
pub(super) fn invalidate_changed_connections(
    config: &mut super::AppConfig,
    previous: &ProviderConfig,
    replacement: &ProviderConfig,
    credentials_changed: bool,
) {
    for connection in config
        .connections
        .iter_mut()
        .filter(|connection| connection.provider_id == previous.id)
    {
        let bucket = Some(connection.bucket.as_str());
        let custom = connection.transport.as_ref();
        if credentials_changed
            || previous.kind != replacement.kind
            || previous.options.account_id != replacement.options.account_id
            || previous.options.transport_for_bucket(bucket, custom)
                != replacement.options.transport_for_bucket(bucket, custom)
        {
            connection.verified = false;
        }
    }
}

#[cfg(test)]
mod tests;
