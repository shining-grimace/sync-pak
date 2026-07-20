use std::{
    future::Future,
    path::Path,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

use crate::{
    archive_upload::ArchiveUploader,
    cancellation::CancellationToken,
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    transfer_paths::LocalTransferRoot,
};

use super::{ArchiveExecutionError, create_and_upload_archive};

struct Uploader {
    fail: bool,
    paths: Mutex<Vec<String>>,
}

impl ArchiveUploader for Uploader {
    type Error = &'static str;

    fn upload(
        &self,
        _: &Path,
        destination: &RelativePath,
        _: &CancellationToken,
        _: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        self.paths.lock().unwrap().push(destination.as_str().into());
        async move { (!self.fail).then_some(()).ok_or("provider failed") }
    }
}

#[test]
fn uploads_a_portably_named_archive_and_removes_its_staging_file() {
    let root = temporary_root();
    std::fs::write(root.join("file.txt"), "contents").unwrap();
    let uploader = Uploader {
        fail: false,
        paths: Mutex::new(vec![]),
    };

    let filename = block_on(create_and_upload_archive(
        &LocalTransferRoot::new(&root),
        &inventory(),
        &root,
        "20260721-123456Z",
        "Café/backup",
        &uploader,
        &CancellationToken::default(),
        4,
    ))
    .unwrap();

    assert_eq!(filename, "20260721-123456Z Café_backup.zip");
    assert_eq!(*uploader.paths.lock().unwrap(), [filename]);
    assert_eq!(temporary_files(&root), 0);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_upload_retains_the_complete_staging_file() {
    let root = temporary_root();
    std::fs::write(root.join("file.txt"), "contents").unwrap();
    let uploader = Uploader {
        fail: true,
        paths: Mutex::new(vec![]),
    };

    let error = block_on(create_and_upload_archive(
        &LocalTransferRoot::new(&root),
        &inventory(),
        &root,
        "20260721-123456Z",
        "Backup",
        &uploader,
        &CancellationToken::default(),
        4,
    ))
    .unwrap_err();

    assert!(matches!(error, ArchiveExecutionError::Upload(_)));
    assert_eq!(temporary_files(&root), 1);
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

fn temporary_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("sync-pak-archive-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    root
}

fn temporary_files(root: &Path) -> usize {
    std::fs::read_dir(root)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with('.'))
        .count()
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test uploader must not suspend"),
    }
}
