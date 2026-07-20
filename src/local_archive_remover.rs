use std::{error::Error, fmt, future::Future};

use crate::{
    archive_prune::ArchiveRemover,
    archive_retention::ArchiveRecord,
    cancellation::CancellationToken,
    inventory::{InventoryError, RelativePath},
    transfer_delete::{TransferDeleteError, delete_local},
    transfer_paths::LocalTransferRoot,
};

/// Removes retention-selected archive ZIPs only from one configured local archive root.
pub struct LocalArchiveRemover {
    root: LocalTransferRoot,
}

impl LocalArchiveRemover {
    pub fn new(root: LocalTransferRoot) -> Self {
        Self { root }
    }
}

impl ArchiveRemover for LocalArchiveRemover {
    type Error = LocalArchiveRemoveError;

    fn remove(&self, archive: &ArchiveRecord) -> impl Future<Output = Result<(), Self::Error>> {
        async move {
            let path =
                RelativePath::new(&archive.location).map_err(LocalArchiveRemoveError::Location)?;
            if !path.as_str().ends_with(".zip") {
                return Err(LocalArchiveRemoveError::NotArchive(path));
            }
            delete_local(&self.root, &path, &CancellationToken::default())
                .map_err(LocalArchiveRemoveError::Delete)
        }
    }
}

#[derive(Debug)]
pub enum LocalArchiveRemoveError {
    Location(InventoryError),
    NotArchive(RelativePath),
    Delete(TransferDeleteError),
}

impl fmt::Display for LocalArchiveRemoveError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Location(error) => {
                write!(formatter, "invalid local archive record location: {error}")
            }
            Self::NotArchive(path) => write!(
                formatter,
                "local archive record is not a ZIP: {}",
                path.as_str()
            ),
            Self::Delete(error) => error.fmt(formatter),
        }
    }
}

impl Error for LocalArchiveRemoveError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Location(error) => Some(error),
            Self::Delete(error) => Some(error),
            Self::NotArchive(_) => None,
        }
    }
}

#[cfg(test)]
#[path = "local_archive_remover_tests.rs"]
mod tests;
