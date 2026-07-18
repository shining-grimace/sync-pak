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
        BucketLister, ObjectDeleter, ObjectLister, ObjectMetadata, ObjectMetadataReader,
        ObjectReader, ObjectWriteMetadata, ObjectWriter, ProviderError, ProviderResult,
        RemoteObject,
    },
    provider_conformance::{
        ConformanceError, ConformancePhase, verify_bucket_listing, verify_object_lifecycle,
    },
};

#[test]
fn lifecycle_checks_and_removes_an_isolated_object() {
    let provider = InMemoryProvider::default();

    assert_eq!(
        block_on(verify_object_lifecycle(
            &provider,
            "isolated-bucket",
            "syncpak-tests/",
            "syncpak-tests/object.txt"
        )),
        Ok(())
    );
    assert_eq!(provider.delete_count.load(Ordering::Relaxed), 1);
    assert!(provider.objects.lock().unwrap().is_empty());
}

#[test]
fn lifecycle_cleans_up_after_an_injected_read_failure() {
    let provider = InMemoryProvider {
        fail_reads: AtomicBool::new(true),
        ..Default::default()
    };

    assert_eq!(
        block_on(verify_object_lifecycle(
            &provider,
            "isolated-bucket",
            "syncpak-tests/",
            "syncpak-tests/object.txt"
        )),
        Err(ConformanceError {
            phase: ConformancePhase::ObjectRead,
            provider_error: ProviderError::Unavailable,
        })
    );
    assert_eq!(provider.delete_count.load(Ordering::Relaxed), 1);
    assert!(provider.objects.lock().unwrap().is_empty());
}

#[test]
fn bucket_listing_requires_the_isolated_bucket() {
    assert_eq!(
        block_on(verify_bucket_listing(
            &InMemoryProvider::default(),
            "isolated-bucket"
        )),
        Ok(())
    );
}

#[derive(Default)]
struct InMemoryProvider {
    delete_count: AtomicU32,
    fail_reads: AtomicBool,
    objects: Mutex<BTreeMap<String, Vec<u8>>>,
    source_modified_times: Mutex<BTreeMap<String, Option<i64>>>,
}

impl BucketLister for InMemoryProvider {
    async fn list_buckets(&self) -> ProviderResult<Vec<String>> {
        Ok(vec!["isolated-bucket".to_owned()])
    }
}

impl ObjectLister for InMemoryProvider {
    async fn list_objects(&self, _: &str, prefix: &str) -> ProviderResult<Vec<RemoteObject>> {
        Ok(self
            .objects
            .lock()
            .unwrap()
            .iter()
            .filter(|(key, _)| key.starts_with(prefix))
            .map(|(key, contents)| RemoteObject {
                key: key.clone(),
                metadata: metadata(contents, None),
            })
            .collect())
    }
}

impl ObjectReader for InMemoryProvider {
    async fn read(&self, _: &str, key: &str) -> ProviderResult<Vec<u8>> {
        if self.fail_reads.load(Ordering::Relaxed) {
            return Err(ProviderError::Unavailable);
        }
        self.objects
            .lock()
            .unwrap()
            .get(key)
            .cloned()
            .ok_or(ProviderError::NotFound)
    }
}

impl ObjectWriter for InMemoryProvider {
    async fn write(&self, _: &str, key: &str, contents: &[u8]) -> ProviderResult<()> {
        self.objects
            .lock()
            .unwrap()
            .insert(key.to_owned(), contents.to_vec());
        self.source_modified_times
            .lock()
            .unwrap()
            .insert(key.to_owned(), None);
        Ok(())
    }

    async fn write_with_metadata(
        &self,
        bucket: &str,
        key: &str,
        contents: &[u8],
        metadata: &ObjectWriteMetadata,
    ) -> ProviderResult<()> {
        self.objects
            .lock()
            .unwrap()
            .insert(key.to_owned(), contents.to_vec());
        self.source_modified_times
            .lock()
            .unwrap()
            .insert(key.to_owned(), metadata.source_modified_unix_seconds);
        let _ = bucket;
        Ok(())
    }
}

impl ObjectMetadataReader for InMemoryProvider {
    async fn metadata(&self, _: &str, key: &str) -> ProviderResult<ObjectMetadata> {
        let contents = self
            .objects
            .lock()
            .unwrap()
            .get(key)
            .cloned()
            .ok_or(ProviderError::NotFound)?;
        let source_modified_unix_seconds = self
            .source_modified_times
            .lock()
            .unwrap()
            .get(key)
            .copied()
            .flatten();
        Ok(metadata(&contents, source_modified_unix_seconds))
    }
}

impl ObjectDeleter for InMemoryProvider {
    async fn delete(&self, _: &str, key: &str) -> ProviderResult<()> {
        self.delete_count.fetch_add(1, Ordering::Relaxed);
        self.objects.lock().unwrap().remove(key);
        self.source_modified_times.lock().unwrap().remove(key);
        Ok(())
    }
}

fn metadata(contents: &[u8], source_modified_unix_seconds: Option<i64>) -> ObjectMetadata {
    ObjectMetadata {
        byte_size: contents.len() as u64,
        modified_unix_seconds: None,
        source_modified_unix_seconds,
        content_type: None,
        entity_tag: None,
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
