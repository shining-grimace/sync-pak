use std::{
    convert::Infallible,
    future::Future,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

use crate::{
    cancellation::CancellationToken,
    comparison::compare,
    configuration::SyncMode,
    destructive_confirmation::DestructiveConfirmation,
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    planning::{Direction, plan},
    transfer_progress::{TransferProgress, TransferProgressObserver},
};

use super::{MirrorExecutionError, MirrorTransfer, execute_confirmed_mirror};

#[derive(Default)]
struct Transfer(Mutex<Vec<String>>);

impl MirrorTransfer for Transfer {
    type Error = Infallible;

    fn copy(
        &self,
        _: Direction,
        path: &RelativePath,
        _: bool,
        _: &CancellationToken,
        _: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async move {
            self.0
                .lock()
                .unwrap()
                .push(format!("copy:{}", path.as_str()));
            Ok(())
        }
    }

    fn delete(
        &self,
        _: Direction,
        path: &RelativePath,
        _: &CancellationToken,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async move {
            self.0
                .lock()
                .unwrap()
                .push(format!("delete:{}", path.as_str()));
            Ok(())
        }
    }
}

struct Observer;
impl TransferProgressObserver for Observer {
    fn on_progress(&self, _: &TransferProgress) {}
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test transfer must not suspend"),
    }
}

fn inventory(paths: &[&str]) -> Inventory {
    Inventory::new(paths.iter().map(|path| {
        InventoryEntry::new(
            RelativePath::new(*path).unwrap(),
            InventoryEntryKind::File,
            1,
            Some(1),
        )
    }))
    .unwrap()
}

#[test]
fn confirmed_mirror_copies_before_deleting() {
    let plan = plan(
        SyncMode::Mirror,
        Direction::Upload,
        &compare(&inventory(&["copy"]), &inventory(&["delete"])),
    )
    .unwrap();
    let confirmation = DestructiveConfirmation::confirm(&plan).unwrap();
    let transfer = Transfer::default();

    block_on(execute_confirmed_mirror(
        &plan,
        Some(&confirmation),
        &transfer,
        &CancellationToken::default(),
        &Observer,
        1,
    ))
    .unwrap();

    assert_eq!(
        transfer.0.lock().unwrap().as_slice(),
        ["copy:copy", "delete:delete"]
    );
}

#[test]
fn destructive_mirror_requires_confirmation() {
    let plan = plan(
        SyncMode::Mirror,
        Direction::Upload,
        &compare(&inventory(&[]), &inventory(&["delete"])),
    )
    .unwrap();

    assert!(matches!(
        block_on(execute_confirmed_mirror(
            &plan,
            None,
            &Transfer::default(),
            &CancellationToken::default(),
            &Observer,
            1
        )),
        Err(MirrorExecutionError::ConfirmationRequired)
    ));
}
