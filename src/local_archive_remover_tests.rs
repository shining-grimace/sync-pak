use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

use crate::{
    archive_prune::ArchiveRemover, archive_retention::ArchiveRecord,
    cancellation::CancellationToken, configuration::ConnectionId,
    transfer_paths::LocalTransferRoot,
};

use super::{LocalArchiveRemoveError, LocalArchiveRemover};

#[test]
fn removes_only_a_zip_beneath_the_configured_local_root() {
    let root = temporary_root();
    std::fs::create_dir(root.join("archives")).unwrap();
    let archive_path = root.join("archives/old.zip");
    std::fs::write(&archive_path, "archive").unwrap();
    let remover = LocalArchiveRemover::new(LocalTransferRoot::new(&root));

    block_on(remover.remove(&record("archives/old.zip"), &CancellationToken::default())).unwrap();

    assert!(!archive_path.exists());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn refuses_non_archive_or_non_normalized_record_locations() {
    let root = temporary_root();
    let remover = LocalArchiveRemover::new(LocalTransferRoot::new(&root));

    assert!(matches!(
        block_on(remover.remove(&record("notes.txt"), &CancellationToken::default())),
        Err(LocalArchiveRemoveError::NotArchive(_))
    ));
    assert!(matches!(
        block_on(remover.remove(&record("../outside.zip"), &CancellationToken::default())),
        Err(LocalArchiveRemoveError::Location(_))
    ));

    std::fs::remove_dir_all(root).unwrap();
}

fn record(location: &str) -> ArchiveRecord {
    ArchiveRecord {
        connection_id: ConnectionId::new(),
        location: location.into(),
        created_at_utc: "20260721-120000Z".into(),
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
        Poll::Pending => panic!("local archive removal must not suspend"),
    }
}
