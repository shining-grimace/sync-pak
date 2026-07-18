use std::{
    future::Future,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

use uuid::Uuid;

use super::upload_from_path;
use crate::provider_capabilities::{ObjectWriteMetadata, ObjectWriter, ProviderResult};

#[derive(Default)]
struct Writer(Mutex<Option<(Vec<u8>, ObjectWriteMetadata)>>);

impl ObjectWriter for Writer {
    async fn write(&self, _: &str, _: &str, contents: &[u8]) -> ProviderResult<()> {
        self.0
            .lock()
            .unwrap()
            .replace((contents.to_vec(), ObjectWriteMetadata::default()));
        Ok(())
    }

    async fn write_with_metadata(
        &self,
        _: &str,
        _: &str,
        contents: &[u8],
        metadata: &ObjectWriteMetadata,
    ) -> ProviderResult<()> {
        self.0
            .lock()
            .unwrap()
            .replace((contents.to_vec(), metadata.clone()));
        Ok(())
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test writer must not suspend"),
    }
}

#[test]
fn uploads_file_contents_with_the_source_modification_time() {
    let source = std::env::temp_dir().join(format!("sync-pak-upload-{}", Uuid::new_v4()));
    std::fs::write(&source, "contents").unwrap();
    let writer = Writer::default();

    block_on(upload_from_path(&writer, "bucket", "key", &source)).unwrap();
    std::fs::remove_file(&source).unwrap();

    let (contents, metadata) = writer.0.lock().unwrap().clone().unwrap();
    assert_eq!(contents, b"contents");
    assert!(metadata.source_modified_unix_seconds.is_some());
}
