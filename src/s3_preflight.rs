//! Read-only preflight collection for S3-compatible providers.

use std::{error::Error, fmt};

use crate::{
    capabilities::ProtectedCredentialStore,
    configuration::{CredentialError, ProviderRepository},
    connection_preflight::collect_connection_preflight,
    inventory_endpoint::{EndpointPreflightError, LocalFolderEndpoint, RemoteFolderEndpoint},
    preflight::{CaseSensitivity, Preflight},
    provider_capabilities::ProviderError,
    run_request::RunRequest,
    s3_transport::S3Transport,
};

/// Collects local and remote inventories for an S3-compatible connection without changing data.
pub async fn collect_s3_connection_preflight<S: ProtectedCredentialStore>(
    request: &RunRequest,
    providers: &ProviderRepository<'_, S>,
    local_case_sensitivity: CaseSensitivity,
) -> Result<Preflight, S3PreflightError> {
    let credentials = providers
        .load_credentials(&request.provider.id)
        .map_err(S3PreflightError::Credentials)?;
    let transport = S3Transport::connect(&request.provider, credentials)
        .await
        .map_err(S3PreflightError::Provider)?;
    let local = LocalFolderEndpoint::new(&request.connection.local_path, local_case_sensitivity);
    let remote = RemoteFolderEndpoint::new(
        &transport,
        &request.connection.bucket,
        &request.connection.remote_path,
    );
    collect_connection_preflight(request, &local, &remote)
        .await
        .map_err(S3PreflightError::Inventory)
}

#[derive(Debug)]
pub enum S3PreflightError {
    Credentials(CredentialError),
    Provider(ProviderError),
    Inventory(EndpointPreflightError),
}

impl fmt::Display for S3PreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Credentials(error) => error.fmt(formatter),
            Self::Provider(error) => error.fmt(formatter),
            Self::Inventory(error) => error.fmt(formatter),
        }
    }
}

impl Error for S3PreflightError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Credentials(error) => Some(error),
            Self::Provider(error) => Some(error),
            Self::Inventory(error) => Some(error),
        }
    }
}
