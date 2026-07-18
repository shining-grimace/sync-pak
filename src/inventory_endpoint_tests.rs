use std::fs;
use std::future::Future;
use std::task::{Context, Poll, Waker};

use uuid::Uuid;

use super::{CaseSensitivity, InventoryEndpoint, LocalFolderEndpoint};

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("local inventory collection must not block"),
    }
}

#[test]
fn local_endpoint_collects_an_inventory_with_explicit_case_sensitivity() {
    let root = std::env::temp_dir().join(format!("sync-pak-endpoint-{}", Uuid::new_v4()));
    fs::create_dir(&root).unwrap();
    fs::write(root.join("é.txt"), "contents").unwrap();
    let endpoint = LocalFolderEndpoint::new(&root, CaseSensitivity::Sensitive);

    let inventory = block_on(endpoint.collect()).unwrap();
    fs::remove_dir_all(&root).unwrap();

    assert_eq!(endpoint.case_sensitivity(), CaseSensitivity::Sensitive);
    assert_eq!(inventory.entries().count(), 1);
}
