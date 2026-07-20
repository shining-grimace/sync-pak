use std::{
    future::Future,
    path::Path,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use crate::{
    archive_download::ArchiveDownloader,
    archive_prune::ArchiveRemover,
    archive_retention::ArchiveRecord,
    cancellation::CancellationToken,
    configuration::ConnectionId,
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    transfer_paths::LocalTransferRoot,
};

use super::download_create_and_prune_archive;

struct Downloader(Arc<Mutex<Vec<String>>>);
struct Remover(Arc<Mutex<Vec<String>>>);

impl ArchiveDownloader for Downloader {
    type Error = &'static str;

    fn download(
        &self,
        source: &RelativePath,
        destination: &Path,
        _: &CancellationToken,
        _: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        self.0
            .lock()
            .unwrap()
            .push(format!("download:{}", source.as_str()));
        let destination = destination.to_owned();
        async move {
            std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
            std::fs::write(destination, "contents").unwrap();
            Ok(())
        }
    }
}

impl ArchiveRemover for Remover {
    type Error = &'static str;

    fn remove(&self, archive: &ArchiveRecord) -> impl Future<Output = Result<(), Self::Error>> {
        self.0
            .lock()
            .unwrap()
            .push(format!("remove:{}", archive.location));
        async { Ok(()) }
    }
}

#[test]
fn stores_the_complete_local_archive_before_pruning_old_records() {
    let root = temporary_root();
    let connection = ConnectionId::new();
    let events = Arc::new(Mutex::new(vec![]));
    let old = record(
        &connection,
        "20260720-120000Z Backup.zip",
        "20260720-120000Z",
    );

    let result = block_on(download_create_and_prune_archive(
        &Downloader(events.clone()),
        &inventory(),
        &root,
        &LocalTransferRoot::new(&root),
        "20260721-120000Z",
        &connection,
        "Backup",
        &[old.clone()],
        1,
        &Remover(events.clone()),
        &CancellationToken::default(),
        5,
    ))
    .unwrap();

    assert!(root.join(&result.archive.location).exists());
    assert_eq!(result.pruned, [old]);
    assert_eq!(
        *events.lock().unwrap(),
        ["download:file.txt", "remove:20260720-120000Z Backup.zip"]
    );
    std::fs::remove_dir_all(root).unwrap();
}

fn inventory() -> Inventory {
    Inventory::new([InventoryEntry::new(
        RelativePath::new("file.txt").unwrap(),
        InventoryEntryKind::File,
        8,
        None,
    )])
    .unwrap()
}

fn record(connection_id: &ConnectionId, location: &str, created_at_utc: &str) -> ArchiveRecord {
    ArchiveRecord {
        connection_id: connection_id.clone(),
        location: location.into(),
        created_at_utc: created_at_utc.into(),
    }
}

fn temporary_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("sync-pak-archive-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    root
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test local archive store must not suspend"),
    }
}
