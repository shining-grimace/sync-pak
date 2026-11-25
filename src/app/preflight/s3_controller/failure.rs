use crate::{
    capabilities::{CapabilityError, CredentialBackend},
    configuration::CredentialError,
    inventory::endpoint::{EndpointInventoryError, EndpointPreflightError},
    providers::s3::preflight::S3PreflightError,
};

#[derive(Clone, Copy)]
pub(super) enum PreflightFailure {
    Credentials(Option<CapabilityError>),
    Provider,
    LocalInventory,
    RemoteInventory,
    Planning,
}

impl From<S3PreflightError> for PreflightFailure {
    fn from(error: S3PreflightError) -> Self {
        match error {
            S3PreflightError::Credentials(CredentialError::ProtectedStore(error)) => {
                Self::Credentials(Some(error))
            }
            S3PreflightError::Credentials(_) => Self::Credentials(None),
            S3PreflightError::Provider(_) => Self::Provider,
            S3PreflightError::Inventory(
                EndpointPreflightError::SourceInventory(EndpointInventoryError::Local(_))
                | EndpointPreflightError::DestinationInventory(EndpointInventoryError::Local(_)),
            ) => Self::LocalInventory,
            S3PreflightError::Inventory(
                EndpointPreflightError::SourceInventory(EndpointInventoryError::Remote(_))
                | EndpointPreflightError::DestinationInventory(EndpointInventoryError::Remote(_)),
            ) => Self::RemoteInventory,
            S3PreflightError::Inventory(EndpointPreflightError::Preflight(_)) => Self::Planning,
        }
    }
}

impl PreflightFailure {
    pub(super) fn diagnostic(self) -> &'static str {
        match self {
            Self::Credentials(Some(CapabilityError::ServiceMissing)) => {
                "saved credential store service unavailable"
            }
            Self::Credentials(Some(CapabilityError::Locked | CapabilityError::UnlockTimedOut)) => {
                "saved credential store locked"
            }
            Self::Credentials(Some(CapabilityError::NotFound)) => "saved credential entry missing",
            Self::Credentials(_) => "saved credential access failed",
            Self::Provider => "provider inventory failed",
            Self::LocalInventory => "local folder inventory failed",
            Self::RemoteInventory => "cloud path inventory failed",
            Self::Planning => "connection preflight planning failed",
        }
    }
    pub(super) fn message(self, backend: CredentialBackend) -> std::borrow::Cow<'static, str> {
        match self {
            Self::Credentials(error) => return credential_message(backend, error).into(),
            Self::Provider => {
                "SyncPak could not reach this provider. Check its credentials, bucket, and network connection."
            }
            Self::LocalInventory => {
                "SyncPak could not read the configured local folder. Check that it exists and that SyncPak has permission to access it."
            }
            Self::RemoteInventory => {
                "SyncPak could not read the configured cloud bucket or remote folder. Check that the cloud path exists and that the provider credentials can access it."
            }
            Self::Planning => {
                "SyncPak read both paths but could not prepare this operation. Review the connection settings, then try again."
            }
        }.into()
    }
}

fn credential_message(backend: CredentialBackend, error: Option<CapabilityError>) -> String {
    let store = backend.name();
    match error {
        Some(CapabilityError::ServiceMissing) => format!(
            "This connection’s credentials are stored in {store}. {store} is unavailable in this session. Restore access to it, then try again."
        ),
        Some(CapabilityError::Locked | CapabilityError::UnlockTimedOut) => format!(
            "{store} is locked. Unlock it to use this connection’s saved credentials, then try again."
        ),
        Some(CapabilityError::NotFound) => format!(
            "{store} does not have a saved credential entry for this connection’s provider."
        ),
        Some(CapabilityError::NotConfigured) => {
            format!("{store} has no default credential collection configured.")
        }
        Some(CapabilityError::AccessDenied) => {
            format!("{store} denied access to this connection’s saved credentials.")
        }
        Some(CapabilityError::Cancelled) => format!("Credential access in {store} was cancelled."),
        Some(CapabilityError::TimedOut) => format!(
            "{store} did not respond when SyncPak requested this connection’s credentials. Try again."
        ),
        _ => format!(
            "SyncPak could not read this connection’s saved credentials from {store}. Check that the store is available, then try again."
        ),
    }
}
