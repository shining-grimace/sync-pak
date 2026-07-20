use std::{
    error::Error,
    fmt, fs,
    future::Future,
    path::{Path, PathBuf},
};

use crate::{
    archive_create::{ArchiveCreateError, create_archive_with_cancellation},
    cancellation::CancellationToken,
    inventory::{Inventory, InventoryEntryKind, RelativePath},
    transfer_paths::LocalTransferRoot,
};

/// Retrieves a remote source file into a private local archive staging tree.
pub trait ArchiveDownloader {
    type Error;

    fn download(
        &self,
        source: &RelativePath,
        destination: &Path,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> impl Future<Output = Result<(), Self::Error>>;
}

/// Creates a local ZIP only after every remote source file has been safely staged.
pub async fn download_and_create_archive<D: ArchiveDownloader>(
    downloader: &D,
    inventory: &Inventory,
    staging_parent: &Path,
    destination: &Path,
    cancellation: &CancellationToken,
    jitter_seed: u64,
) -> Result<(), ArchiveDownloadError<D::Error>> {
    let staging = StagingTree::new(staging_parent).map_err(ArchiveDownloadError::Local)?;
    for entry in inventory.entries() {
        if cancellation.check().is_err() {
            return Err(ArchiveDownloadError::Cancelled);
        }
        let path = staging.root.resolve(&entry.path);
        match &entry.kind {
            InventoryEntryKind::Directory => {
                fs::create_dir_all(path).map_err(ArchiveDownloadError::Local)?
            }
            InventoryEntryKind::File => downloader
                .download(&entry.path, &path, cancellation, jitter_seed)
                .await
                .map_err(ArchiveDownloadError::Download)?,
            InventoryEntryKind::Symlink { .. } => {
                return Err(ArchiveDownloadError::UnsupportedSymlink(entry.path.clone()));
            }
        }
    }
    create_archive_with_cancellation(&staging.root, inventory, destination, cancellation)
        .map_err(ArchiveDownloadError::Create)
}

struct StagingTree {
    root: LocalTransferRoot,
    path: PathBuf,
}

impl StagingTree {
    fn new(parent: &Path) -> Result<Self, std::io::Error> {
        fs::create_dir_all(parent)?;
        let path = parent.join(format!(".syncpak-archive-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&path)?;
        Ok(Self {
            root: LocalTransferRoot::new(&path),
            path,
        })
    }
}

impl Drop for StagingTree {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.path);
    }
}

#[derive(Debug)]
pub enum ArchiveDownloadError<E> {
    Cancelled,
    Download(E),
    Create(ArchiveCreateError),
    Local(std::io::Error),
    UnsupportedSymlink(RelativePath),
}

impl<E: fmt::Display> fmt::Display for ArchiveDownloadError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Cancelled => formatter.write_str("archive download was cancelled"),
            Self::Download(error) => {
                write!(formatter, "could not download archive source: {error}")
            }
            Self::Create(error) => error.fmt(formatter),
            Self::Local(error) => write!(formatter, "could not prepare archive staging: {error}"),
            Self::UnsupportedSymlink(path) => write!(
                formatter,
                "remote archive source contains an unsupported symlink: {}",
                path.as_str()
            ),
        }
    }
}

impl<E: Error + 'static> Error for ArchiveDownloadError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Download(error) => Some(error),
            Self::Create(error) => Some(error),
            Self::Local(error) => Some(error),
            Self::Cancelled | Self::UnsupportedSymlink(_) => None,
        }
    }
}

#[cfg(test)]
#[path = "archive_download_tests.rs"]
mod tests;
