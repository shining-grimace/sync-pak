use std::{
    collections::BTreeMap,
    future::Future,
    sync::{
        Mutex,
        atomic::{AtomicBool, AtomicU32, Ordering},
    },
    task::{Context, Poll, Waker},
};

use crate::{
    provider_capabilities::{
        MultipartUpload, MultipartUploadRequest, MultipartUploader, ObjectDeleter, ObjectReader,
        ProviderError, ProviderResult, UploadedPart,
    },
    provider_conformance::{ConformanceError, ConformancePhase},
    provider_multipart_conformance::verify_multipart_lifecycle,
};

#[test]
fn multipart_lifecycle_completes_cleans_up_and_aborts() {
    let provider = InMemoryMultipartProvider::default();

    assert_eq!(
        block_on(verify_multipart_lifecycle(
            &provider,
            "isolated-bucket",
            "syncpak-tests/multipart"
        )),
        Ok(())
    );
    assert_eq!(provider.abort_count.load(Ordering::Relaxed), 1);
    assert!(provider.objects.lock().unwrap().is_empty());
    assert!(provider.uploads.lock().unwrap().is_empty());
}

#[test]
fn failed_part_upload_is_aborted() {
    let provider = InMemoryMultipartProvider {
        fail_second_part: AtomicBool::new(true),
        ..Default::default()
    };

    assert_eq!(
        block_on(verify_multipart_lifecycle(
            &provider,
            "isolated-bucket",
            "syncpak-tests/multipart"
        )),
        Err(ConformanceError {
            phase: ConformancePhase::MultipartPartUpload,
            provider_error: ProviderError::Unavailable,
        })
    );
    assert_eq!(provider.abort_count.load(Ordering::Relaxed), 1);
    assert!(provider.uploads.lock().unwrap().is_empty());
}

#[derive(Default)]
struct InMemoryMultipartProvider {
    abort_count: AtomicU32,
    fail_second_part: AtomicBool,
    next_upload_id: AtomicU32,
    objects: Mutex<BTreeMap<String, Vec<u8>>>,
    uploads: Mutex<BTreeMap<String, Vec<Vec<u8>>>>,
}

impl MultipartUploader for InMemoryMultipartProvider {
    async fn begin_multipart_upload(
        &self,
        _: &MultipartUploadRequest,
    ) -> ProviderResult<MultipartUpload> {
        let id = self
            .next_upload_id
            .fetch_add(1, Ordering::Relaxed)
            .to_string();
        self.uploads.lock().unwrap().insert(id.clone(), Vec::new());
        Ok(MultipartUpload { id })
    }

    async fn upload_part(
        &self,
        _: &str,
        _: &str,
        upload: &MultipartUpload,
        part_number: u32,
        contents: &[u8],
    ) -> ProviderResult<UploadedPart> {
        if part_number == 2 && self.fail_second_part.load(Ordering::Relaxed) {
            return Err(ProviderError::Unavailable);
        }
        self.uploads
            .lock()
            .unwrap()
            .get_mut(&upload.id)
            .ok_or(ProviderError::NotFound)?
            .push(contents.to_vec());
        Ok(UploadedPart {
            part_number,
            entity_tag: format!("etag-{part_number}"),
        })
    }

    async fn complete_multipart_upload(
        &self,
        _: &str,
        key: &str,
        upload: &MultipartUpload,
        _: &[UploadedPart],
    ) -> ProviderResult<()> {
        let parts = self
            .uploads
            .lock()
            .unwrap()
            .remove(&upload.id)
            .ok_or(ProviderError::NotFound)?;
        self.objects
            .lock()
            .unwrap()
            .insert(key.to_owned(), parts.concat());
        Ok(())
    }

    async fn abort_multipart_upload(
        &self,
        _: &str,
        _: &str,
        upload: &MultipartUpload,
    ) -> ProviderResult<()> {
        self.abort_count.fetch_add(1, Ordering::Relaxed);
        self.uploads.lock().unwrap().remove(&upload.id);
        Ok(())
    }
}

impl ObjectReader for InMemoryMultipartProvider {
    async fn read(&self, _: &str, key: &str) -> ProviderResult<Vec<u8>> {
        self.objects
            .lock()
            .unwrap()
            .get(key)
            .cloned()
            .ok_or(ProviderError::NotFound)
    }
}

impl ObjectDeleter for InMemoryMultipartProvider {
    async fn delete(&self, _: &str, key: &str) -> ProviderResult<()> {
        self.objects.lock().unwrap().remove(key);
        Ok(())
    }
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(value) => value,
        Poll::Pending => panic!("the in-memory provider should not suspend"),
    }
}
