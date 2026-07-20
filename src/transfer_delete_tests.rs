use std::{
    future::Future,
    sync::{
        Mutex,
        atomic::{AtomicU8, Ordering},
    },
    task::{Context, Poll, Waker},
};

use crate::{
    cancellation::CancellationToken,
    inventory::RelativePath,
    provider_capabilities::{ObjectDeleter, ProviderError, ProviderResult},
    retry::{RetryPolicy, RetrySleeper},
    transfer_paths::{LocalTransferRoot, RemoteTransferPrefix},
};

use super::{delete_local, delete_remote, delete_remote_with_retry_and_cancellation};

#[derive(Default)]
struct Provider(Mutex<Vec<String>>);

impl ObjectDeleter for Provider {
    async fn delete(&self, _: &str, key: &str) -> ProviderResult<()> {
        self.0.lock().unwrap().push(key.into());
        Ok(())
    }
}

struct FlakyProvider(AtomicU8);

impl ObjectDeleter for FlakyProvider {
    async fn delete(&self, _: &str, _: &str) -> ProviderResult<()> {
        (self.0.fetch_add(1, Ordering::Relaxed) != 0)
            .then_some(())
            .ok_or(ProviderError::Unavailable)
    }
}

#[derive(Default)]
struct Sleeper(Mutex<usize>);

impl RetrySleeper for Sleeper {
    async fn sleep(&self, _: std::time::Duration) {
        *self.0.lock().unwrap() += 1;
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test provider must not suspend"),
    }
}

#[test]
fn removes_local_files_and_empty_directories() {
    let root = std::env::temp_dir().join(format!("sync-pak-delete-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("file"), "contents").unwrap();
    std::fs::create_dir(root.join("empty")).unwrap();
    let root = LocalTransferRoot::new(&root);

    delete_local(
        &root,
        &RelativePath::new("file").unwrap(),
        &CancellationToken::default(),
    )
    .unwrap();
    delete_local(
        &root,
        &RelativePath::new("empty").unwrap(),
        &CancellationToken::default(),
    )
    .unwrap();

    assert!(!root.resolve(&RelativePath::new("file").unwrap()).exists());
    assert!(!root.resolve(&RelativePath::new("empty").unwrap()).exists());
    std::fs::remove_dir(root.as_path()).unwrap();
}

#[test]
fn removes_prefixed_remote_objects() {
    let provider = Provider::default();
    block_on(delete_remote(
        &provider,
        "bucket",
        &RemoteTransferPrefix::new("sync").unwrap(),
        &RelativePath::new("file").unwrap(),
        &CancellationToken::default(),
    ))
    .unwrap();
    assert_eq!(provider.0.lock().unwrap().as_slice(), ["sync/file"]);
}

#[test]
fn retries_an_unavailable_remote_delete() {
    let provider = FlakyProvider(AtomicU8::new(0));
    let sleeper = Sleeper::default();
    block_on(delete_remote_with_retry_and_cancellation(
        &provider,
        "bucket",
        &RemoteTransferPrefix::new("sync").unwrap(),
        &RelativePath::new("file").unwrap(),
        &RetryPolicy::default(),
        &sleeper,
        1,
        &CancellationToken::default(),
    ))
    .unwrap();
    assert_eq!(provider.0.load(Ordering::Relaxed), 2);
    assert_eq!(*sleeper.0.lock().unwrap(), 1);
}
