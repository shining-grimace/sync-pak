//! Versioned, non-secret configuration persisted independently from credentials.

mod connections;
mod credentials;
mod diagnostics;
pub mod lists;
mod model;
mod providers;
pub mod run_guard;
mod store;
mod transport;
mod validation;

pub use connections::{ConnectionDraft, ConnectionError, ConnectionRepository};
pub use credentials::{CredentialError, ProviderCredentials, ProviderRepository};
pub use diagnostics::{DiagnosticLog, StructuredError};
pub use model::{
    AppConfig, AppearancePreference, ConnectionConfig, ConnectionId, CredentialReference,
    ProviderConfig, ProviderId, ProviderKind, ProviderOptions, SyncMode,
};
pub use model::{CURRENT_SCHEMA_VERSION, validate_run_policy};
pub use providers::{DeletedProvider, ProviderDraft};
pub use store::{ConfigStore, ConfigurationError};
pub use transport::TransportSettings;
pub use validation::ValidationErrors;
