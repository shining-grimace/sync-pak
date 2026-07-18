use std::{
    future::Future,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    task::{Context, Poll, Waker},
};

use super::{MultipartUploadError, upload_parts};
use crate::provider_capabilities::{
    MultipartUpload, MultipartUploadRequest, MultipartUploader, ProviderError, ProviderResult,
    UploadedPart,
};

#[derive(Default)]
struct Provider {
    aborts: AtomicU32,
    fail_second_part: AtomicBool,
    parts: Mutex<Vec<Vec<u8>>>,
}

impl MultipartUploader for Provider {
    async fn begin_multipart_upload(
        &self,
        _: &MultipartUploadRequest,
    ) -> ProviderResult<MultipartUpload> {
        Ok(MultipartUpload { id: "id".into() })
    }

    async fn upload_part(
        &self,
        _: &str,
        _: &str,
        _: &MultipartUpload,
        part_number: u32,
        contents: &[u8],
    ) -> ProviderResult<UploadedPart> {
        if part_number == 2 && self.fail_second_part.load(Ordering::Relaxed) {
            return Err(ProviderError::Unavailable);
        }
        self.parts.lock().unwrap().push(contents.to_vec());
        Ok(UploadedPart {
            part_number,
            entity_tag: format!("{part_number}"),
        })
    }

    async fn complete_multipart_upload(
        &self,
        _: &str,
        _: &str,
        _: &MultipartUpload,
        _: &[UploadedPart],
    ) -> ProviderResult<()> {
        Ok(())
    }

    async fn abort_multipart_upload(
        &self,
        _: &str,
        _: &str,
        _: &MultipartUpload,
    ) -> ProviderResult<()> {
        self.aborts.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

fn request() -> MultipartUploadRequest {
    MultipartUploadRequest {
        bucket: "bucket".into(),
        key: "key".into(),
        content_type: None,
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
fn completes_after_uploading_parts_in_order() {
    let provider = Provider::default();
    block_on(upload_parts(&provider, &request(), &[b"one", b"two"])).unwrap();

    assert_eq!(
        provider.parts.lock().unwrap().as_slice(),
        [b"one".to_vec(), b"two".to_vec()]
    );
    assert_eq!(provider.aborts.load(Ordering::Relaxed), 0);
}

#[test]
fn aborts_when_a_part_fails() {
    let provider = Provider {
        fail_second_part: AtomicBool::new(true),
        ..Default::default()
    };

    assert_eq!(
        block_on(upload_parts(&provider, &request(), &[b"one", b"two"])),
        Err(MultipartUploadError::Provider {
            error: ProviderError::Unavailable,
            abort_error: None
        })
    );
    assert_eq!(provider.aborts.load(Ordering::Relaxed), 1);
}
