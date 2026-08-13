//! Read-only preflight collection for S3-compatible providers.

use std::{error::Error, fmt, path::Path};

use crate::{
    capabilities::ProtectedCredentialStore,
    configuration::{CredentialError, ProviderRepository},
    inventory::endpoint::{
        EndpointInventoryError, EndpointPreflightError, InventoryEndpoint,
        InventorySnapshotEndpoint, LocalFolderEndpoint,
    },
    inventory::remote::{inventory_from_objects, normalize_prefix},
    operations::request::RunRequest,
    preflight::connection::collect_connection_preflight,
    preflight::{CaseSensitivity, Preflight},
    providers::capabilities::ProviderError,
    providers::s3::transport::S3Transport,
    sync_cache::SyncCache,
};

/// Collects local and remote inventories for an S3-compatible connection without changing data.
pub async fn collect_s3_connection_preflight<S: ProtectedCredentialStore>(
    request: &RunRequest,
    providers: &ProviderRepository<'_, S>,
    local_case_sensitivity: CaseSensitivity,
    configuration_path: &Path,
) -> Result<Preflight, S3PreflightError> {
    let credentials = providers
        .load_credentials(&request.provider.id)
        .map_err(S3PreflightError::Credentials)?;
    let transport = S3Transport::connect(&request.provider, credentials)
        .await
        .map_err(S3PreflightError::Provider)?;
    let local = LocalFolderEndpoint::new(&request.connection.local_path, local_case_sensitivity);
    let local_inventory = local.collect().await.map_err(|error| {
        S3PreflightError::Inventory(EndpointPreflightError::SourceInventory(error))
    })?;
    let prefix = normalize_prefix(&request.connection.remote_path)
        .map_err(EndpointInventoryError::Remote)
        .map_err(EndpointPreflightError::DestinationInventory)
        .map_err(S3PreflightError::Inventory)?;
    let cache = SyncCache::for_configuration(configuration_path);
    let namespace = cache.as_ref().map(|cache| cache.namespace(request));
    let snapshot = cache
        .as_ref()
        .zip(namespace.as_ref())
        .map(|(cache, namespace)| cache.snapshot(namespace));
    let objects = transport
        .comparison_objects(
            &request.connection.bucket,
            &prefix,
            &local_inventory,
            cache
                .as_ref()
                .zip(namespace.as_ref())
                .zip(snapshot.as_ref()),
        )
        .await
        .map_err(crate::inventory::remote::RemoteInventoryError::Provider)
        .map_err(EndpointInventoryError::Remote)
        .map_err(EndpointPreflightError::DestinationInventory)
        .map_err(S3PreflightError::Inventory)?;
    let remote_inventory = inventory_from_objects(&prefix, objects)
        .map_err(EndpointInventoryError::Remote)
        .map_err(EndpointPreflightError::DestinationInventory)
        .map_err(S3PreflightError::Inventory)?;
    let local = InventorySnapshotEndpoint::new(local_inventory, local.case_sensitivity());
    let remote = InventorySnapshotEndpoint::new(remote_inventory, CaseSensitivity::Sensitive);
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
