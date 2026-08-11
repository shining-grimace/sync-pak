use std::error::Error;
use std::fmt;
use std::future::Future;

use crate::configuration::SyncMode;
use crate::inventory::Inventory;
use crate::inventory::local::LocalInventoryError;
use crate::inventory::remote::{RemoteInventoryError, list_remote_inventory};
use crate::operations::transfer::paths::LocalTransferRoot;
use crate::preflight::planning::Direction;
use crate::preflight::{CaseSensitivity, Preflight, PreflightError, preflight};
use crate::providers::capabilities::ObjectLister;

pub trait InventoryEndpoint {
    fn case_sensitivity(&self) -> CaseSensitivity;

    fn collect(&self) -> impl Future<Output = Result<Inventory, EndpointInventoryError>> + Send;
}

#[derive(Clone, Debug)]
pub struct LocalFolderEndpoint {
    root: LocalTransferRoot,
    case_sensitivity: CaseSensitivity,
}

impl LocalFolderEndpoint {
    pub fn new(root: &str, case_sensitivity: CaseSensitivity) -> Self {
        Self {
            root: LocalTransferRoot::from_config(root),
            case_sensitivity,
        }
    }
}

impl InventoryEndpoint for LocalFolderEndpoint {
    fn case_sensitivity(&self) -> CaseSensitivity {
        self.case_sensitivity
    }

    async fn collect(&self) -> Result<Inventory, EndpointInventoryError> {
        self.root.inventory().map_err(EndpointInventoryError::Local)
    }
}

pub struct RemoteFolderEndpoint<'a, L: ObjectLister + Sync + ?Sized> {
    lister: &'a L,
    bucket: String,
    prefix: String,
}

impl<'a, L: ObjectLister + Sync + ?Sized> RemoteFolderEndpoint<'a, L> {
    pub fn new(lister: &'a L, bucket: impl Into<String>, prefix: impl Into<String>) -> Self {
        Self {
            lister,
            bucket: bucket.into(),
            prefix: prefix.into(),
        }
    }
}

impl<L: ObjectLister + Sync + ?Sized> InventoryEndpoint for RemoteFolderEndpoint<'_, L> {
    fn case_sensitivity(&self) -> CaseSensitivity {
        CaseSensitivity::Sensitive
    }

    async fn collect(&self) -> Result<Inventory, EndpointInventoryError> {
        list_remote_inventory(self.lister, &self.bucket, &self.prefix)
            .await
            .map_err(EndpointInventoryError::Remote)
    }
}

pub async fn collect_preflight<S: InventoryEndpoint, D: InventoryEndpoint>(
    mode: SyncMode,
    direction: Direction,
    source: &S,
    destination: &D,
) -> Result<Preflight, EndpointPreflightError> {
    let source_inventory = source
        .collect()
        .await
        .map_err(EndpointPreflightError::SourceInventory)?;
    let destination_inventory = destination
        .collect()
        .await
        .map_err(EndpointPreflightError::DestinationInventory)?;
    preflight(
        mode,
        direction,
        &source_inventory,
        source.case_sensitivity(),
        &destination_inventory,
        destination.case_sensitivity(),
    )
    .map_err(EndpointPreflightError::Preflight)
}

#[derive(Debug)]
pub enum EndpointInventoryError {
    Local(LocalInventoryError),
    Remote(RemoteInventoryError),
}

impl fmt::Display for EndpointInventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Local(error) => error.fmt(formatter),
            Self::Remote(error) => error.fmt(formatter),
        }
    }
}

impl Error for EndpointInventoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Local(error) => Some(error),
            Self::Remote(error) => Some(error),
        }
    }
}

#[derive(Debug)]
pub enum EndpointPreflightError {
    SourceInventory(EndpointInventoryError),
    DestinationInventory(EndpointInventoryError),
    Preflight(PreflightError),
}

impl fmt::Display for EndpointPreflightError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceInventory(error) => {
                write!(formatter, "could not inventory source: {error}")
            }
            Self::DestinationInventory(error) => {
                write!(formatter, "could not inventory destination: {error}")
            }
            Self::Preflight(error) => error.fmt(formatter),
        }
    }
}

impl Error for EndpointPreflightError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::SourceInventory(error) | Self::DestinationInventory(error) => Some(error),
            Self::Preflight(error) => Some(error),
        }
    }
}

#[cfg(test)]
mod tests;
