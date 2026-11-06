use super::*;
use crate::{
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    preflight::{
        CaseSensitivity,
        mirror_selection::{MirrorChoice, MirrorSelection},
        preflight,
    },
};
use std::{
    convert::Infallible,
    future::Future,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

#[derive(Default)]
struct Transfer(Mutex<Vec<(Direction, String, bool)>>);
impl MirrorTransfer for Transfer {
    type Error = Infallible;
    async fn copy(
        &self,
        direction: Direction,
        path: &RelativePath,
        overwrite: bool,
        _: &CancellationToken,
        _: u64,
    ) -> Result<(), Self::Error> {
        self.0
            .lock()
            .unwrap()
            .push((direction, path.as_str().into(), overwrite));
        Ok(())
    }
    async fn delete(
        &self,
        direction: Direction,
        path: &RelativePath,
        _: &CancellationToken,
    ) -> Result<(), Self::Error> {
        self.0
            .lock()
            .unwrap()
            .push((direction, format!("delete:{}", path.as_str()), false));
        Ok(())
    }
}
struct Observer;
impl TransferProgressObserver for Observer {
    fn on_progress(&self, _: &TransferProgress) {}
}
fn block_on<F: Future>(future: F) -> F::Output {
    let mut context = Context::from_waker(Waker::noop());
    match std::pin::pin!(future).as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test transfer must not suspend"),
    }
}
fn inventory(entries: &[(&str, u64)]) -> Inventory {
    Inventory::new(entries.iter().map(|(path, size)| {
        InventoryEntry::new(
            RelativePath::new(*path).unwrap(),
            InventoryEntryKind::File,
            *size,
            Some(1),
        )
    }))
    .unwrap()
}

#[test]
fn mixed_mirror_routes_each_copy_overwrite_and_delete_to_its_approved_side() {
    let local = inventory(&[("upload", 1), ("local-delete", 1), ("conflict", 2)]);
    let remote = inventory(&[("download", 1), ("remote-delete", 1), ("conflict", 4)]);
    let original = preflight(
        SyncMode::Mirror,
        Direction::BothWays,
        &local,
        CaseSensitivity::Sensitive,
        &remote,
        CaseSensitivity::Sensitive,
    )
    .unwrap();
    let mut selection = MirrorSelection::new(original);
    for (path, choice) in [
        ("upload", MirrorChoice::Keep),
        ("download", MirrorChoice::Keep),
        ("local-delete", MirrorChoice::Delete),
        ("remote-delete", MirrorChoice::Delete),
        ("conflict", MirrorChoice::UseRemote),
    ] {
        selection.choose(&RelativePath::new(path).unwrap(), Some(choice));
    }
    let selected = selection.selected().unwrap();
    let confirmation = DestructiveConfirmation::confirm(selected.plan()).unwrap();
    let transfer = Transfer::default();
    block_on(execute_confirmed_mirror(
        selected.plan(),
        Some(&confirmation),
        &transfer,
        &CancellationToken::default(),
        &Observer,
        0,
    ))
    .unwrap();
    assert_eq!(
        *transfer.0.lock().unwrap(),
        vec![
            (Direction::Download, "conflict".into(), true),
            (Direction::Download, "download".into(), false),
            (Direction::Upload, "upload".into(), false),
            (Direction::Download, "delete:local-delete".into(), false),
            (Direction::Upload, "delete:remote-delete".into(), false),
        ]
    );
}

#[test]
fn download_review_preserves_local_remote_orientation() {
    let local = inventory(&[("local-only", 1), ("conflict", 2)]);
    let remote = inventory(&[("remote-only", 1), ("conflict", 3)]);
    let original = preflight(
        SyncMode::Mirror,
        Direction::Download,
        &remote,
        CaseSensitivity::Sensitive,
        &local,
        CaseSensitivity::Sensitive,
    )
    .unwrap();
    let mut selection = MirrorSelection::new(original);
    selection.choose(
        &RelativePath::new("local-only").unwrap(),
        Some(MirrorChoice::Delete),
    );
    selection.choose(
        &RelativePath::new("remote-only").unwrap(),
        Some(MirrorChoice::Keep),
    );
    selection.choose(
        &RelativePath::new("conflict").unwrap(),
        Some(MirrorChoice::UseRemote),
    );
    let selected = selection.selected().unwrap();
    let confirmation = DestructiveConfirmation::confirm(selected.plan()).unwrap();
    let transfer = Transfer::default();
    block_on(execute_confirmed_mirror(
        selected.plan(),
        Some(&confirmation),
        &transfer,
        &CancellationToken::default(),
        &Observer,
        0,
    ))
    .unwrap();
    assert!(
        transfer
            .0
            .lock()
            .unwrap()
            .iter()
            .all(|(direction, _, _)| *direction == Direction::Download)
    );
}
