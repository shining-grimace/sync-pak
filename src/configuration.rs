//! Versioned, non-secret configuration persisted independently from credentials.

mod connections;
mod credentials;
mod diagnostics;
mod model;
mod providers;
mod store;
mod validation;

pub use connections::{ConnectionError, ConnectionRepository};
pub use credentials::{CredentialError, ProviderCredentials, ProviderRepository};
pub use diagnostics::{DiagnosticReport, StructuredError};
pub use model::CURRENT_SCHEMA_VERSION;
pub use model::{
    AppConfig, ConnectionConfig, ConnectionDraft, ConnectionId, CredentialReference,
    ProviderConfig, ProviderId, ProviderKind, ProviderOptions, SyncMode,
};
pub use providers::{DeletedProvider, ProviderDraft};
pub use store::{ConfigStore, ConfigurationError};
pub use validation::ValidationErrors;
