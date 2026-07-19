use std::{
    future::Future,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

use uuid::Uuid;

use crate::{
    cancellation::CancellationToken,
    inventory::RelativePath,
    provider_capabilities::{ObjectReader, ObjectWriteMetadata, ObjectWriter, ProviderResult},
    retry::{RetryPolicy, RetrySleeper},
    transfer_paths::{LocalTransferRoot, RemoteTransferPrefix},
};

use super::LocalRemoteTransfer;

#[derive(Default)]
struct Provider {
    writes: Mutex<Vec<(String, Vec<u8>)>>,
}

impl ObjectWriter for Provider {
    async fn write(&self, _: &str, _: &str, _: &[u8]) -> ProviderResult<()> {
        Ok(())
    }

    async fn write_with_metadata(
        &self,
        _: &str,
        key: &str,
        contents: &[u8],
        _: &ObjectWriteMetadata,
    ) -> ProviderResult<()> {
        self.writes
            .lock()
            .unwrap()
            .push((key.into(), contents.to_vec()));
        Ok(())
    }
}

impl ObjectReader for Provider {
    async fn read(&self, _: &str, _: &str) -> ProviderResult<Vec<u8>> {
        Ok(b"remote".to_vec())
    }
}

struct NoopSleeper;

impl RetrySleeper for NoopSleeper {
    async fn sleep(&self, _: std::time::Duration) {}
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

fn transfer<'a>(
    provider: &'a Provider,
    root: &'a std::path::Path,
    policy: &'a RetryPolicy,
) -> LocalRemoteTransfer<'a, Provider, NoopSleeper> {
    static SLEEPER: NoopSleeper = NoopSleeper;
    LocalRemoteTransfer::new(
        provider,
        "bucket",
        LocalTransferRoot::new(root),
        RemoteTransferPrefix::new("sync").unwrap(),
        policy,
        &SLEEPER,
    )
}

#[test]
fn uploads_a_relative_local_file_to_its_prefixed_key() {
    let root = std::env::temp_dir().join(format!("sync-pak-transfer-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("photo.jpg"), b"local").unwrap();
    let provider = Provider::default();
    let policy = RetryPolicy::default();

    block_on(transfer(&provider, &root, &policy).upload(
        &RelativePath::new("photo.jpg").unwrap(),
        &CancellationToken::default(),
        1,
    ))
    .unwrap();

    assert_eq!(
        provider.writes.lock().unwrap().as_slice(),
        [("sync/photo.jpg".into(), b"local".to_vec())]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn downloads_a_relative_key_to_its_local_root() {
    let root = std::env::temp_dir().join(format!("sync-pak-transfer-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let provider = Provider::default();
    let policy = RetryPolicy::default();

    block_on(transfer(&provider, &root, &policy).download(
        &RelativePath::new("folder/photo.jpg").unwrap(),
        &CancellationToken::default(),
        1,
    ))
    .unwrap();

    assert_eq!(
        std::fs::read(root.join("folder/photo.jpg")).unwrap(),
        b"remote"
    );
    std::fs::remove_dir_all(&root).unwrap();
}
