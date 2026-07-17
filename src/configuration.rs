//! Versioned, non-secret configuration persisted independently from credentials.

mod credentials;
mod diagnostics;
mod model;
mod store;
mod validation;

pub use credentials::{CredentialError, ProviderCredentials, ProviderRepository};
pub use diagnostics::{DiagnosticReport, StructuredError};
pub use model::CURRENT_SCHEMA_VERSION;
pub use model::{
    AppConfig, ConnectionConfig, ConnectionId, CredentialReference, ProviderConfig, ProviderId,
    ProviderKind, ProviderOptions, SyncMode,
};
pub use store::{ConfigStore, ConfigurationError};
pub use validation::ValidationErrors;
