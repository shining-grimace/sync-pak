use std::{
    future::Future,
    path::Path,
    sync::{Arc, Mutex},
    task::{Context, Poll, Waker},
};

use crate::{
    archive_prune::ArchiveRemover,
    archive_retention::ArchiveRecord,
    archive_upload::ArchiveUploader,
    cancellation::CancellationToken,
    configuration::ConnectionId,
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    transfer_paths::LocalTransferRoot,
};

use super::{ArchiveStoreError, create_upload_and_prune_archive};

struct Uploader {
    fail: bool,
    events: Arc<Mutex<Vec<String>>>,
}
struct Remover(Arc<Mutex<Vec<String>>>);

impl ArchiveUploader for Uploader {
    type Error = &'static str;

    fn upload(
        &self,
        _: &Path,
        _: &RelativePath,
        _: &CancellationToken,
        _: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        self.events.lock().unwrap().push("upload".into());
        async move { (!self.fail).then_some(()).ok_or("upload failed") }
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
fn prunes_only_after_the_new_archive_has_been_stored() {
    let root = temporary_root();
    std::fs::write(root.join("file.txt"), "contents").unwrap();
    let connection = ConnectionId::new();
    let events = Arc::new(Mutex::new(vec![]));
    let old = record(&connection, "old.zip", "20260720-120000Z");

    let result = block_on(create_upload_and_prune_archive(
        &LocalTransferRoot::new(&root),
        &inventory(),
        &root,
        "20260721-120000Z",
        &connection,
        "Backup",
        &[old.clone()],
        1,
        &Uploader {
            fail: false,
            events: events.clone(),
        },
        &Remover(events.clone()),
        &CancellationToken::default(),
        3,
    ))
    .unwrap();

    assert_eq!(result.pruned, [old]);
    assert_eq!(*events.lock().unwrap(), ["upload", "remove:old.zip"]);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_storage_never_starts_retention_pruning() {
    let root = temporary_root();
    std::fs::write(root.join("file.txt"), "contents").unwrap();
    let connection = ConnectionId::new();
    let events = Arc::new(Mutex::new(vec![]));

    assert!(matches!(
        block_on(create_upload_and_prune_archive(
            &LocalTransferRoot::new(&root),
            &inventory(),
            &root,
            "20260721-120000Z",
            &connection,
            "Backup",
            &[],
            1,
            &Uploader {
                fail: true,
                events: events.clone()
            },
            &Remover(events.clone()),
            &CancellationToken::default(),
            3,
        )),
        Err(ArchiveStoreError::Store(_))
    ));

    assert_eq!(*events.lock().unwrap(), ["upload"]);
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
        Poll::Pending => panic!("test archive store must not suspend"),
    }
}
