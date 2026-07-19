use std::{
    convert::Infallible,
    future::Future,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

use crate::{
    cancellation::CancellationToken,
    inventory::RelativePath,
    planning::{Direction, Endpoint, PlannedAction},
    transfer_progress::{TransferProgress, TransferProgressObserver},
};

use super::{AddOnlyTransfer, execute_add_only_actions};

#[derive(Default)]
struct Transfer(Mutex<Vec<String>>);

impl AddOnlyTransfer for Transfer {
    type Error = Infallible;

    fn upload(
        &self,
        path: &RelativePath,
        _: &CancellationToken,
        _: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async move {
            self.0
                .lock()
                .unwrap()
                .push(format!("upload:{}", path.as_str()));
            Ok(())
        }
    }

    fn download(
        &self,
        path: &RelativePath,
        _: &CancellationToken,
        _: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        async move {
            self.0
                .lock()
                .unwrap()
                .push(format!("download:{}", path.as_str()));
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

fn copy(path: &str, from: Endpoint, to: Endpoint) -> PlannedAction {
    PlannedAction::Copy {
        path: RelativePath::new(path).unwrap(),
        from,
        to,
    }
}

#[test]
fn both_ways_dispatches_each_copy_in_its_planned_direction() {
    let transfer = Transfer::default();
    let actions = [
        copy("local", Endpoint::Source, Endpoint::Destination),
        copy("remote", Endpoint::Destination, Endpoint::Source),
    ];

    block_on(execute_add_only_actions(
        Direction::BothWays,
        &actions,
        &transfer,
        &CancellationToken::default(),
        &Observer,
        1,
    ))
    .unwrap();

    assert_eq!(
        transfer.0.lock().unwrap().as_slice(),
        ["upload:local", "download:remote"]
    );
}

#[test]
fn download_direction_dispatches_source_to_destination_as_a_download() {
    let transfer = Transfer::default();

    block_on(execute_add_only_actions(
        Direction::Download,
        &[copy("remote", Endpoint::Source, Endpoint::Destination)],
        &transfer,
        &CancellationToken::default(),
        &Observer,
        1,
    ))
    .unwrap();

    assert_eq!(transfer.0.lock().unwrap().as_slice(), ["download:remote"]);
}
