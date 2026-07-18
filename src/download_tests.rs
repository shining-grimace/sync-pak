use std::{
    fs,
    future::Future,
    task::{Context, Poll, Waker},
};

use uuid::Uuid;

use super::{DownloadError, download_to_path};
use crate::provider_capabilities::{ObjectReader, ProviderError, ProviderResult};

struct Reader(ProviderResult<Vec<u8>>);

impl ObjectReader for Reader {
    async fn read(&self, _: &str, _: &str) -> ProviderResult<Vec<u8>> {
        self.0.clone()
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test reader must not suspend"),
    }
}

#[test]
fn replaces_a_download_destination_only_after_a_successful_provider_read() {
    let directory = std::env::temp_dir().join(format!("sync-pak-download-{}", Uuid::new_v4()));
    fs::create_dir(&directory).unwrap();
    let destination = directory.join("file.txt");
    fs::write(&destination, "old").unwrap();

    block_on(download_to_path(
        &Reader(Ok(b"new".to_vec())),
        "bucket",
        "key",
        &destination,
    ))
    .unwrap();

    assert_eq!(fs::read_to_string(&destination).unwrap(), "new");
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn provider_failures_leave_an_existing_destination_intact() {
    let directory = std::env::temp_dir().join(format!("sync-pak-download-{}", Uuid::new_v4()));
    fs::create_dir(&directory).unwrap();
    let destination = directory.join("file.txt");
    fs::write(&destination, "old").unwrap();

    let error = block_on(download_to_path(
        &Reader(Err(ProviderError::Unavailable)),
        "bucket",
        "key",
        &destination,
    ))
    .unwrap_err();

    assert!(matches!(
        error,
        DownloadError::Provider(ProviderError::Unavailable)
    ));
    assert_eq!(fs::read_to_string(&destination).unwrap(), "old");
    fs::remove_dir_all(&directory).unwrap();
}
