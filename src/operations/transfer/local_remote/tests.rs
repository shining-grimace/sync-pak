use std::{
    future::Future,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

use uuid::Uuid;

use crate::{
    configuration::ConnectionId,
    inventory::RelativePath,
    operations::archive::prune::ArchiveRemover,
    operations::archive::retention::ArchiveRecord,
    operations::archive::upload::ArchiveUploader,
    operations::cancellation::CancellationToken,
    operations::retry::{RetryPolicy, RetrySleeper},
    operations::transfer::paths::{LocalTransferRoot, RemoteTransferPrefix},
    providers::capabilities::{
        MultipartUpload, MultipartUploadRequest, MultipartUploader, ObjectDeleter, ObjectMetadata,
        ObjectMetadataReader, ObjectPrefixChecker, ObjectReader, ObjectWriteMetadata, ObjectWriter,
        ProviderError, ProviderResult, ReadObject, UploadedPart,
    },
    sync_cache::{CacheNamespace, RemoteIdentity, RemoteObservation, SyncCache},
};

use super::{LocalRemoteTransfer, LocalRemoteTransferError};
#[derive(Default)]
struct Provider {
    writes: Mutex<Vec<(String, Vec<u8>)>>,
    multipart_keys: Mutex<Vec<String>>,
    deletes: Mutex<Vec<String>>,
    missing_reads: Mutex<Vec<String>>,
    existing_prefixes: Mutex<Vec<String>>,
    source_modified: Mutex<Option<i64>>,
    metadata_reads: Mutex<usize>,
}

impl ObjectWriter for Provider {
    async fn write_with_metadata(
        &self,
        _: &str,
        key: &str,
        contents: &[u8],
        metadata: &ObjectWriteMetadata,
    ) -> ProviderResult<()> {
        *self.source_modified.lock().unwrap() = metadata.source_modified_unix_seconds;
        self.writes
            .lock()
            .unwrap()
            .push((key.into(), contents.to_vec()));
        Ok(())
    }
}

impl ObjectReader for Provider {
    async fn read(&self, _: &str, key: &str) -> ProviderResult<Vec<u8>> {
        if self
            .missing_reads
            .lock()
            .unwrap()
            .iter()
            .any(|missing| missing == key)
        {
            Err(ProviderError::NotFound)
        } else {
            Ok(b"remote".to_vec())
        }
    }

    async fn read_with_metadata(&self, bucket: &str, key: &str) -> ProviderResult<ReadObject> {
        Ok(ReadObject {
            contents: self.read(bucket, key).await?,
            metadata: Some(self.metadata(bucket, key).await?),
        })
    }
}

impl ObjectMetadataReader for Provider {
    async fn metadata(&self, _: &str, _: &str) -> ProviderResult<ObjectMetadata> {
        *self.metadata_reads.lock().unwrap() += 1;
        Ok(ObjectMetadata {
            byte_size: 6,
            modified_unix_seconds: Some(100),
            source_modified_unix_seconds: *self.source_modified.lock().unwrap(),
            content_type: None,
            entity_tag: Some("etag".into()),
        })
    }
}

impl ObjectPrefixChecker for Provider {
    async fn prefix_exists(&self, _: &str, prefix: &str) -> ProviderResult<bool> {
        Ok(self
            .existing_prefixes
            .lock()
            .unwrap()
            .iter()
            .any(|existing| existing == prefix))
    }
}

impl ObjectDeleter for Provider {
    async fn delete(&self, _: &str, key: &str) -> ProviderResult<()> {
        self.deletes.lock().unwrap().push(key.into());
        Ok(())
    }
}

impl MultipartUploader for Provider {
    async fn begin_multipart_upload(
        &self,
        request: &MultipartUploadRequest,
    ) -> ProviderResult<MultipartUpload> {
        self.multipart_keys
            .lock()
            .unwrap()
            .push(request.key.clone());
        Ok(MultipartUpload { id: "id".into() })
    }

    async fn upload_part(
        &self,
        _: &str,
        _: &str,
        _: &MultipartUpload,
        part_number: u32,
        _: &[u8],
    ) -> ProviderResult<UploadedPart> {
        Ok(UploadedPart {
            part_number,
            entity_tag: part_number.to_string(),
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
        Ok(())
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
fn records_a_verified_baseline_after_a_successful_upload() {
    let root = std::env::temp_dir().join(format!("sync-pak-transfer-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("photo.jpg"), b"local!").unwrap();
    let cache = SyncCache::for_configuration(&root.join("state/config.json")).unwrap();
    let namespace = CacheNamespace::from_stored("test-namespace".into());
    let provider = Provider::default();
    let policy = RetryPolicy::default();
    let transfer = transfer(&provider, &root, &policy).with_cache(cache.clone(), namespace.clone());

    block_on(transfer.upload_auto(
        &RelativePath::new("photo.jpg").unwrap(),
        &CancellationToken::default(),
        1,
    ))
    .unwrap();

    let snapshot = cache.snapshot(&namespace);
    assert!(snapshot.baseline("photo.jpg").is_some());
    assert!(snapshot.observation("bucket", "sync/photo.jpg").is_some());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn records_an_equal_sized_pre_existing_pair_as_accepted() {
    let root = std::env::temp_dir().join(format!("sync-pak-transfer-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("photo.jpg"), b"local!").unwrap();
    let cache = SyncCache::for_configuration(&root.join("state/config.json")).unwrap();
    let namespace = CacheNamespace::from_stored("test-namespace".into());
    cache.record_observations(&[RemoteObservation {
        namespace: namespace.clone(),
        bucket: "bucket".into(),
        key: "sync/photo.jpg".into(),
        identity: RemoteIdentity {
            byte_size: 6,
            modified_unix_seconds: Some(100),
            entity_tag: Some("etag".into()),
        },
        source_modified_unix_seconds: None,
    }]);
    let provider = Provider::default();
    let policy = RetryPolicy::default();
    let transfer = transfer(&provider, &root, &policy).with_cache(cache.clone(), namespace.clone());

    block_on(transfer.record_accepted_pair(&RelativePath::new("photo.jpg").unwrap()));

    let snapshot = cache.snapshot(&namespace);
    let baseline = snapshot.baseline("photo.jpg").unwrap();
    assert_eq!(baseline.local.byte_size, 6);
    assert_eq!(baseline.remote.byte_size, 6);
    assert!(snapshot.observation("bucket", "sync/photo.jpg").is_some());
    assert_eq!(*provider.metadata_reads.lock().unwrap(), 0);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn does_not_accept_a_pre_existing_pair_with_different_sizes() {
    let root = std::env::temp_dir().join(format!("sync-pak-transfer-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("photo.jpg"), b"short").unwrap();
    let cache = SyncCache::for_configuration(&root.join("state/config.json")).unwrap();
    let namespace = CacheNamespace::from_stored("test-namespace".into());
    let provider = Provider::default();
    let policy = RetryPolicy::default();
    let transfer = transfer(&provider, &root, &policy).with_cache(cache.clone(), namespace.clone());

    block_on(transfer.record_accepted_pair(&RelativePath::new("photo.jpg").unwrap()));

    assert!(cache.snapshot(&namespace).baseline("photo.jpg").is_none());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn uploads_an_empty_directory_as_a_prefixed_marker() {
    let root = std::env::temp_dir().join(format!("sync-pak-transfer-{}", Uuid::new_v4()));
    std::fs::create_dir_all(root.join("empty")).unwrap();
    let provider = Provider::default();
    let policy = RetryPolicy::default();

    block_on(transfer(&provider, &root, &policy).upload_auto(
        &RelativePath::new("empty").unwrap(),
        &CancellationToken::default(),
        1,
    ))
    .unwrap();

    assert_eq!(
        provider.writes.lock().unwrap().as_slice(),
        [("sync/empty/".into(), Vec::new())]
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
    let modified = std::fs::metadata(root.join("folder/photo.jpg"))
        .unwrap()
        .modified()
        .unwrap()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    assert_eq!(modified, 100);
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn downloads_a_directory_marker_to_the_local_root() {
    let root = std::env::temp_dir().join(format!("sync-pak-transfer-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let provider = Provider::default();
    provider
        .missing_reads
        .lock()
        .unwrap()
        .push("sync/empty".into());
    provider
        .existing_prefixes
        .lock()
        .unwrap()
        .push("sync/empty/".into());
    let policy = RetryPolicy::default();

    block_on(transfer(&provider, &root, &policy).download(
        &RelativePath::new("empty").unwrap(),
        &CancellationToken::default(),
        1,
    ))
    .unwrap();

    assert!(root.join("empty").is_dir());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn downloads_an_implicit_remote_directory_without_a_marker_object() {
    let root = std::env::temp_dir().join(format!("sync-pak-transfer-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let provider = Provider::default();
    provider
        .missing_reads
        .lock()
        .unwrap()
        .push("sync/folder".into());
    provider
        .existing_prefixes
        .lock()
        .unwrap()
        .push("sync/folder/".into());
    let policy = RetryPolicy::default();

    block_on(transfer(&provider, &root, &policy).download(
        &RelativePath::new("folder").unwrap(),
        &CancellationToken::default(),
        1,
    ))
    .unwrap();

    assert!(root.join("folder").is_dir());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn missing_remote_file_without_descendants_remains_a_failure() {
    let root = std::env::temp_dir().join(format!("sync-pak-transfer-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let provider = Provider::default();
    provider
        .missing_reads
        .lock()
        .unwrap()
        .push("sync/missing.txt".into());
    let policy = RetryPolicy::default();

    let result = block_on(transfer(&provider, &root, &policy).download(
        &RelativePath::new("missing.txt").unwrap(),
        &CancellationToken::default(),
        1,
    ));

    assert!(matches!(
        result,
        Err(LocalRemoteTransferError::Download(
            crate::operations::transfer::download::DownloadError::Provider(ProviderError::NotFound)
        ))
    ));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn downloads_a_remote_file_to_an_archive_staging_path() {
    let root = std::env::temp_dir().join(format!("sync-pak-transfer-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let staging = root.join("staging/file.txt");
    let provider = Provider::default();
    let policy = RetryPolicy::default();

    block_on(transfer(&provider, &root, &policy).download_path(
        &RelativePath::new("file.txt").unwrap(),
        &staging,
        &CancellationToken::default(),
        2,
    ))
    .unwrap();

    assert_eq!(std::fs::read(&staging).unwrap(), b"remote");
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn uploads_a_threshold_sized_file_with_multipart() {
    let root = std::env::temp_dir().join(format!("sync-pak-transfer-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(
        root.join("large.bin"),
        vec![
            0_u8;
            crate::operations::transfer::upload_strategy::MULTIPART_THRESHOLD_BYTES as usize
        ],
    )
    .unwrap();
    let provider = Provider::default();
    let policy = RetryPolicy::default();

    block_on(transfer(&provider, &root, &policy).upload_auto(
        &RelativePath::new("large.bin").unwrap(),
        &CancellationToken::default(),
        1,
    ))
    .unwrap();

    assert_eq!(
        provider.multipart_keys.lock().unwrap().as_slice(),
        ["sync/large.bin"]
    );
    assert!(provider.writes.lock().unwrap().is_empty());
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn uploads_an_archive_staging_file_to_its_prefixed_destination() {
    let root = std::env::temp_dir().join(format!("sync-pak-transfer-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let archive = root.join("archive.tmp");
    std::fs::write(&archive, b"zip").unwrap();
    let provider = Provider::default();
    let policy = RetryPolicy::default();

    block_on(ArchiveUploader::upload(
        &transfer(&provider, &root, &policy),
        &archive,
        &RelativePath::new("archives/backup.zip").unwrap(),
        &CancellationToken::default(),
        3,
    ))
    .unwrap();

    assert_eq!(
        provider.writes.lock().unwrap().as_slice(),
        [("sync/archives/backup.zip".into(), b"zip".to_vec())]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn removes_a_validated_prefixed_archive_record() {
    let root = std::env::temp_dir().join(format!("sync-pak-transfer-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let provider = Provider::default();
    let policy = RetryPolicy::default();
    let archive = ArchiveRecord {
        connection_id: ConnectionId::new(),
        location: "archives/old.zip".into(),
        created_at_utc: "20260721-120000Z".into(),
    };

    block_on(ArchiveRemover::remove(
        &transfer(&provider, &root, &policy),
        &archive,
        &CancellationToken::default(),
    ))
    .unwrap();

    assert_eq!(
        provider.deletes.lock().unwrap().as_slice(),
        ["sync/archives/old.zip"]
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cancelled_archive_retention_does_not_start_a_remote_delete() {
    let root = std::env::temp_dir().join(format!("sync-pak-transfer-{}", Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    let provider = Provider::default();
    let policy = RetryPolicy::default();
    let cancellation = CancellationToken::default();
    cancellation.cancel();
    let archive = ArchiveRecord {
        connection_id: ConnectionId::new(),
        location: "archives/old.zip".into(),
        created_at_utc: "20260721-120000Z".into(),
    };

    assert!(matches!(
        block_on(ArchiveRemover::remove(
            &transfer(&provider, &root, &policy),
            &archive,
            &cancellation,
        )),
        Err(LocalRemoteTransferError::Delete(
            crate::operations::transfer::delete::TransferDeleteError::Cancelled
        ))
    ));

    assert!(provider.deletes.lock().unwrap().is_empty());
    std::fs::remove_dir_all(&root).unwrap();
}
