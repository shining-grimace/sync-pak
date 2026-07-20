use std::{
    future::Future,
    path::Path,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

use crate::{
    archive_create::stage_archive,
    cancellation::CancellationToken,
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    transfer_paths::LocalTransferRoot,
};

use super::{ArchiveUploadError, ArchiveUploader, upload_staged_archive};

struct Uploader {
    fail: bool,
    destinations: Mutex<Vec<String>>,
}

impl ArchiveUploader for Uploader {
    type Error = &'static str;

    fn upload(
        &self,
        source: &Path,
        destination: &RelativePath,
        _: &CancellationToken,
        _: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        let source_exists = source.exists();
        self.destinations
            .lock()
            .unwrap()
            .push(destination.as_str().into());
        async move {
            if self.fail || !source_exists {
                Err("provider failed")
            } else {
                Ok(())
            }
        }
    }
}

fn staged(root: &Path) -> crate::archive_create::StagedArchive {
    std::fs::write(root.join("file.txt"), "contents").unwrap();
    let inventory = Inventory::new([InventoryEntry::new(
        RelativePath::new("file.txt").unwrap(),
        InventoryEntryKind::File,
        8,
        None,
    )])
    .unwrap();
    stage_archive(
        &LocalTransferRoot::new(root),
        &inventory,
        root,
        "archive.zip".as_ref(),
    )
    .unwrap()
}

#[test]
fn confirmed_upload_removes_the_staged_archive() {
    let root = tempfile();
    let archive = staged(&root);
    let path = archive.path().to_owned();
    let uploader = Uploader {
        fail: false,
        destinations: Mutex::new(vec![]),
    };

    block_on(upload_staged_archive(
        &uploader,
        archive,
        &RelativePath::new("archives/archive.zip").unwrap(),
        &CancellationToken::default(),
        1,
    ))
    .unwrap();

    assert!(!path.exists());
    assert_eq!(
        *uploader.destinations.lock().unwrap(),
        ["archives/archive.zip"]
    );
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_upload_keeps_the_staged_archive_for_recovery() {
    let root = tempfile();
    let archive = staged(&root);
    let expected_path = archive.path().to_owned();
    let uploader = Uploader {
        fail: true,
        destinations: Mutex::new(vec![]),
    };

    let error = block_on(upload_staged_archive(
        &uploader,
        archive,
        &RelativePath::new("archives/archive.zip").unwrap(),
        &CancellationToken::default(),
        1,
    ))
    .unwrap_err();

    match error {
        ArchiveUploadError::Upload { staged, .. } => assert_eq!(staged.path(), expected_path),
        _ => panic!("expected retained archive upload failure"),
    }
    assert!(expected_path.exists());
    std::fs::remove_dir_all(root).unwrap();
}

fn tempfile() -> std::path::PathBuf {
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
        Poll::Pending => panic!("test uploader must not suspend"),
    }
}
