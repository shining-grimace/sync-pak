use std::{
    fs,
    future::Future,
    sync::{
        Mutex,
        atomic::{AtomicU8, Ordering},
    },
    task::{Context, Poll, Waker},
};

use uuid::Uuid;

use super::{
    DownloadError, download_to_path, download_to_path_with_retry,
    download_to_path_with_retry_and_cancellation,
};
use crate::cancellation::CancellationToken;
use crate::provider_capabilities::{ObjectReader, ProviderError, ProviderResult};
use crate::retry::{RetryPolicy, RetrySleeper};

struct Reader(ProviderResult<Vec<u8>>);

impl ObjectReader for Reader {
    async fn read(&self, _: &str, _: &str) -> ProviderResult<Vec<u8>> {
        self.0.clone()
    }
}

struct FlakyReader(AtomicU8);

impl ObjectReader for FlakyReader {
    async fn read(&self, _: &str, _: &str) -> ProviderResult<Vec<u8>> {
        (self.0.fetch_add(1, Ordering::Relaxed) != 0)
            .then_some(b"contents".to_vec())
            .ok_or(ProviderError::Unavailable)
    }
}

#[derive(Default)]
struct RecordingSleeper(Mutex<Vec<std::time::Duration>>);

impl RetrySleeper for RecordingSleeper {
    async fn sleep(&self, delay: std::time::Duration) {
        self.0.lock().unwrap().push(delay);
    }
}

struct CancellingSleeper(CancellationToken);

impl RetrySleeper for CancellingSleeper {
    async fn sleep(&self, _: std::time::Duration) {
        self.0.cancel();
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

#[test]
fn retries_a_transient_read_failure_before_replacing_the_destination() {
    let directory = std::env::temp_dir().join(format!("sync-pak-download-{}", Uuid::new_v4()));
    fs::create_dir(&directory).unwrap();
    let destination = directory.join("file.txt");
    let sleeper = RecordingSleeper::default();

    block_on(download_to_path_with_retry(
        &FlakyReader(AtomicU8::new(0)),
        "bucket",
        "key",
        &destination,
        &RetryPolicy::default(),
        &sleeper,
        1,
    ))
    .unwrap();

    assert_eq!(fs::read_to_string(&destination).unwrap(), "contents");
    assert_eq!(sleeper.0.lock().unwrap().len(), 1);
    fs::remove_dir_all(&directory).unwrap();
}

#[test]
fn cancellation_after_a_retry_delay_preserves_the_destination() {
    let directory = std::env::temp_dir().join(format!("sync-pak-download-{}", Uuid::new_v4()));
    fs::create_dir(&directory).unwrap();
    let destination = directory.join("file.txt");
    fs::write(&destination, "old").unwrap();
    let cancellation = CancellationToken::default();
    let reader = FlakyReader(AtomicU8::new(0));

    assert!(matches!(
        block_on(download_to_path_with_retry_and_cancellation(
            &reader,
            "bucket",
            "key",
            &destination,
            &RetryPolicy::default(),
            &CancellingSleeper(cancellation.clone()),
            1,
            &cancellation,
        )),
        Err(DownloadError::Cancelled)
    ));

    assert_eq!(reader.0.load(Ordering::Relaxed), 1);
    assert_eq!(fs::read_to_string(&destination).unwrap(), "old");
    fs::remove_dir_all(&directory).unwrap();
}
