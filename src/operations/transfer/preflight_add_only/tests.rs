use std::{
    convert::Infallible,
    future::Future,
    task::{Context, Poll, Waker},
};

use crate::{
    configuration::SyncMode,
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    operations::cancellation::CancellationToken,
    operations::transfer::modes::add_only::AddOnlyTransfer,
    operations::transfer::progress::{TransferProgress, TransferProgressObserver},
    preflight::planning::Direction,
    preflight::{CaseSensitivity, preflight},
};

use super::{PreflightExecutionError, execute_current_add_only};

struct Transfer;

impl AddOnlyTransfer for Transfer {
    type Error = Infallible;

    fn upload(
        &self,
        _: &RelativePath,
        _: &CancellationToken,
        _: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async { Ok(()) }
    }

    fn download(
        &self,
        _: &RelativePath,
        _: &CancellationToken,
        _: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async { Ok(()) }
    }

    fn accept_existing(&self, _: &RelativePath) -> impl Future<Output = Result<(), Self::Error>> {
        async { Ok(()) }
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

#[test]
fn stale_inventory_blocks_execution_before_any_transfer() {
    let original = Inventory::default();
    let preflight = preflight(
        SyncMode::AddOnly,
        Direction::Upload,
        &original,
        CaseSensitivity::Sensitive,
        &original,
        CaseSensitivity::Sensitive,
    )
    .unwrap();
    let changed = Inventory::new([InventoryEntry::new(
        RelativePath::new("new-file").unwrap(),
        InventoryEntryKind::File,
        1,
        None,
    )])
    .unwrap();

    let result = block_on(execute_current_add_only(
        &preflight,
        &changed,
        &original,
        &Transfer,
        &CancellationToken::default(),
        &Observer,
        1,
    ));

    assert!(matches!(result, Err(PreflightExecutionError::Stale)));
}
