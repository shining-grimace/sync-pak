use std::{
    convert::Infallible,
    future::Future,
    task::{Context, Poll, Waker},
};

use crate::{
    cancellation::CancellationToken,
    configuration::SyncMode,
    destructive_confirmation::DestructiveConfirmation,
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    mirror_execution::MirrorTransfer,
    planning::Direction,
    preflight::{CaseSensitivity, preflight},
    transfer_progress::{TransferProgress, TransferProgressObserver},
};

use super::{CurrentMirrorExecutionError, execute_current_confirmed_mirror};

struct Transfer;

impl MirrorTransfer for Transfer {
    type Error = Infallible;

    fn copy(
        &self,
        _: Direction,
        _: &RelativePath,
        _: bool,
        _: &CancellationToken,
        _: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async { Ok(()) }
    }
    fn delete(
        &self,
        _: Direction,
        _: &RelativePath,
        _: &CancellationToken,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async { Ok(()) }
    }
}

struct Observer;
impl TransferProgressObserver for Observer {
    fn on_progress(&self, _: &TransferProgress) {}
}

#[test]
fn stale_mirror_inventory_blocks_even_a_confirmed_destructive_plan() {
    let original_source = Inventory::default();
    let original_destination = inventory("delete");
    let preflight = preflight(
        SyncMode::Mirror,
        Direction::Upload,
        &original_source,
        CaseSensitivity::Sensitive,
        &original_destination,
        CaseSensitivity::Sensitive,
    )
    .unwrap();
    let confirmation = DestructiveConfirmation::confirm(preflight.plan()).unwrap();
    let changed_source = inventory("new");

    assert!(matches!(
        block_on(execute_current_confirmed_mirror(
            &preflight,
            &changed_source,
            &original_destination,
            Some(&confirmation),
            &Transfer,
            &CancellationToken::default(),
            &Observer,
            1,
        )),
        Err(CurrentMirrorExecutionError::Stale)
    ));
}

fn inventory(path: &str) -> Inventory {
    Inventory::new([InventoryEntry::new(
        RelativePath::new(path).unwrap(),
        InventoryEntryKind::File,
        1,
        None,
    )])
    .unwrap()
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test mirror transfer must not suspend"),
    }
}
