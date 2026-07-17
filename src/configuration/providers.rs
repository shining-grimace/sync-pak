use super::{ConnectionId, ProviderConfig, ProviderKind, ProviderOptions};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderDraft {
    pub name: String,
    pub kind: ProviderKind,
    pub options: ProviderOptions,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DeletedProvider {
    pub provider: ProviderConfig,
    pub dependent_connection_ids: Vec<ConnectionId>,
}
