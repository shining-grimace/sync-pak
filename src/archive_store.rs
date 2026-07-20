use std::{error::Error, fmt, path::Path};

use crate::{
    archive_execution::{ArchiveExecutionError, create_and_upload_archive},
    archive_prune::{ArchivePruneError, ArchiveRemover, prune_archives},
    archive_retention::ArchiveRecord,
    archive_upload::ArchiveUploader,
    cancellation::CancellationToken,
    configuration::ConnectionId,
    inventory::Inventory,
    transfer_paths::LocalTransferRoot,
};

/// A newly stored archive and any older archives removed by its retention policy.
#[derive(Debug)]
pub struct ArchiveStoreResult {
    pub archive: ArchiveRecord,
    pub pruned: Vec<ArchiveRecord>,
}

/// Stores a remote archive, then applies its connection-scoped retention policy.
pub async fn create_upload_and_prune_archive<U: ArchiveUploader, R: ArchiveRemover>(
    source_root: &LocalTransferRoot,
    inventory: &Inventory,
    staging_directory: &Path,
    timestamp: &str,
    connection_id: &ConnectionId,
    connection_name: &str,
    existing: &[ArchiveRecord],
    keep_last: u32,
    uploader: &U,
    remover: &R,
    cancellation: &CancellationToken,
    jitter_seed: u64,
) -> Result<ArchiveStoreResult, ArchiveStoreError<U::Error, R::Error>> {
    let filename = create_and_upload_archive(
        source_root,
        inventory,
        staging_directory,
        timestamp,
        connection_name,
        uploader,
        cancellation,
        jitter_seed,
    )
    .await
    .map_err(ArchiveStoreError::Store)?;
    let archive = ArchiveRecord {
        connection_id: connection_id.clone(),
        location: filename,
        created_at_utc: timestamp.into(),
    };
    let pruned = prune_archives(remover, existing, &archive, keep_last, cancellation)
        .await
        .map_err(|error| ArchiveStoreError::Prune {
            archive: archive.clone(),
            error,
        })?;
    Ok(ArchiveStoreResult { archive, pruned })
}

#[derive(Debug)]
pub enum ArchiveStoreError<U, R> {
    Store(ArchiveExecutionError<U>),
    Prune {
        archive: ArchiveRecord,
        error: ArchivePruneError<R>,
    },
}

impl<U: fmt::Display, R: fmt::Display> fmt::Display for ArchiveStoreError<U, R> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => error.fmt(formatter),
            Self::Prune { archive, error } => write!(
                formatter,
                "archive {} was stored, but retention pruning failed: {error}",
                archive.location
            ),
        }
    }
}

impl<U: Error + 'static, R: Error + 'static> Error for ArchiveStoreError<U, R> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Store(error) => Some(error),
            Self::Prune { error, .. } => Some(error),
        }
    }
}

#[cfg(test)]
#[path = "archive_store_tests.rs"]
mod tests;
