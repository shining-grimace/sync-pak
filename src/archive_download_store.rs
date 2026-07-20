use std::{error::Error, fmt, path::Path};

use crate::{
    archive_download::{ArchiveDownloadError, ArchiveDownloader, download_and_create_archive},
    archive_naming::{ArchiveNameError, archive_filename},
    archive_prune::{ArchivePruneError, ArchiveRemover, prune_archives},
    archive_retention::ArchiveRecord,
    archive_store::ArchiveStoreResult,
    cancellation::CancellationToken,
    configuration::ConnectionId,
    inventory::{Inventory, InventoryError, RelativePath},
    transfer_paths::LocalTransferRoot,
};

/// Stores a downloaded archive locally, then applies its connection-scoped retention policy.
pub async fn download_create_and_prune_archive<D: ArchiveDownloader, R: ArchiveRemover>(
    downloader: &D,
    inventory: &Inventory,
    staging_parent: &Path,
    destination_root: &LocalTransferRoot,
    timestamp: &str,
    connection_id: &ConnectionId,
    connection_name: &str,
    existing: &[ArchiveRecord],
    keep_last: u32,
    remover: &R,
    cancellation: &CancellationToken,
    jitter_seed: u64,
) -> Result<ArchiveStoreResult, ArchiveDownloadStoreError<D::Error, R::Error>> {
    let filename =
        archive_filename(timestamp, connection_name).map_err(ArchiveDownloadStoreError::Name)?;
    let relative = RelativePath::new(filename.clone()).map_err(ArchiveDownloadStoreError::Path)?;
    download_and_create_archive(
        downloader,
        inventory,
        staging_parent,
        &destination_root.resolve(&relative),
        cancellation,
        jitter_seed,
    )
    .await
    .map_err(ArchiveDownloadStoreError::Store)?;
    let archive = ArchiveRecord {
        connection_id: connection_id.clone(),
        location: filename,
        created_at_utc: timestamp.into(),
    };
    let pruned = prune_archives(remover, existing, &archive, keep_last, cancellation)
        .await
        .map_err(|error| ArchiveDownloadStoreError::Prune {
            archive: archive.clone(),
            error,
        })?;
    Ok(ArchiveStoreResult { archive, pruned })
}

#[derive(Debug)]
pub enum ArchiveDownloadStoreError<D, R> {
    Name(ArchiveNameError),
    Path(InventoryError),
    Store(ArchiveDownloadError<D>),
    Prune {
        archive: ArchiveRecord,
        error: ArchivePruneError<R>,
    },
}

impl<D: fmt::Display, R: fmt::Display> fmt::Display for ArchiveDownloadStoreError<D, R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Name(error) => error.fmt(formatter),
            Self::Path(error) => error.fmt(formatter),
            Self::Store(error) => error.fmt(formatter),
            Self::Prune { archive, error } => write!(
                formatter,
                "archive {} was stored, but retention pruning failed: {error}",
                archive.location
            ),
        }
    }
}

impl<D: Error + 'static, R: Error + 'static> Error for ArchiveDownloadStoreError<D, R> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Name(error) => Some(error),
            Self::Path(error) => Some(error),
            Self::Store(error) => Some(error),
            Self::Prune { error, .. } => Some(error),
        }
    }
}

#[cfg(test)]
#[path = "archive_download_store_tests.rs"]
mod tests;
