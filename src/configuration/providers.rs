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
