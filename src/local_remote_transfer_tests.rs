use std::{
    future::Future,
    sync::Mutex,
    task::{Context, Poll, Waker},
};

use uuid::Uuid;

use crate::{
    archive_prune::ArchiveRemover,
    archive_retention::ArchiveRecord,
    archive_upload::ArchiveUploader,
    cancellation::CancellationToken,
    configuration::ConnectionId,
    inventory::RelativePath,
    provider_capabilities::{
        MultipartUpload, MultipartUploadRequest, MultipartUploader, ObjectDeleter, ObjectReader,
        ObjectWriteMetadata, ObjectWriter, ProviderResult, UploadedPart,
    },
    retry::{RetryPolicy, RetrySleeper},
    transfer_paths::{LocalTransferRoot, RemoteTransferPrefix},
};

use super::{LocalRemoteTransfer, LocalRemoteTransferError};
#[derive(Default)]
struct Provider {
    writes: Mutex<Vec<(String, Vec<u8>)>>,
    multipart_keys: Mutex<Vec<String>>,
    deletes: Mutex<Vec<String>>,
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
        vec![0_u8; crate::upload_strategy::MULTIPART_THRESHOLD_BYTES as usize],
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
            crate::transfer_delete::TransferDeleteError::Cancelled
        ))
    ));

    assert!(provider.deletes.lock().unwrap().is_empty());
    std::fs::remove_dir_all(&root).unwrap();
}
