use std::{future::Future, sync::Mutex, task::Poll};

use crate::providers::capabilities::{
    ObjectPrefixChecker, ObjectPrefixCreator, ProviderError, ProviderResult,
};

use super::{
    ConnectionVerificationError, create_remote_folder, verify_local_folder, verify_remote_folder,
};

struct Prefix(bool);

impl ObjectPrefixChecker for Prefix {
    async fn prefix_exists(&self, _: &str, _: &str) -> ProviderResult<bool> {
        Ok(self.0)
    }
}

#[test]
fn local_verification_distinguishes_missing_and_non_directory_paths() {
    let root = std::env::temp_dir().join(format!(
        "sync-pak-connection-verification-{}",
        std::process::id()
    ));
    let missing = root.join("missing");
    assert_eq!(
        verify_local_folder(missing.to_str().unwrap()),
        Err(ConnectionVerificationError::LocalFolderMissing)
    );
    assert_eq!(
        verify_local_folder(std::env::temp_dir().to_str().unwrap()),
        Ok(())
    );
    assert_eq!(
        verify_local_folder(std::env::current_exe().unwrap().to_str().unwrap()),
        Err(ConnectionVerificationError::LocalPathNotDirectory)
    );
}

#[test]
fn remote_verification_reports_a_missing_cloud_path() {
    assert_eq!(
        block_on(verify_remote_folder(&Prefix(false), "bucket", "photos")),
        Err(ConnectionVerificationError::RemoteFolderMissing)
    );
    assert_eq!(
        block_on(verify_remote_folder(&Prefix(false), "bucket", "")),
        Err(ConnectionVerificationError::Provider(
            ProviderError::NotFound
        ))
    );
}

struct RemoteStorage {
    exists: ProviderResult<bool>,
    writes: Mutex<Vec<(String, String)>>,
}

impl ObjectPrefixChecker for RemoteStorage {
    async fn prefix_exists(&self, _: &str, _: &str) -> ProviderResult<bool> {
        self.exists.clone()
    }
}

impl ObjectPrefixCreator for RemoteStorage {
    async fn create_prefix(&self, bucket: &str, prefix: &str) -> ProviderResult<()> {
        self.writes
            .lock()
            .unwrap()
            .push((bucket.into(), prefix.into()));
        Ok(())
    }
}

#[test]
fn creation_writes_only_the_normalized_missing_prefix() {
    let storage = RemoteStorage {
        exists: Ok(false),
        writes: Mutex::default(),
    };
    assert_eq!(
        block_on(create_remote_folder(&storage, "bucket", "photos/2026/")),
        Ok(())
    );
    assert_eq!(
        *storage.writes.lock().unwrap(),
        [("bucket".into(), "photos/2026/".into())]
    );
}

#[test]
fn creation_skips_existing_prefix_and_rejects_bucket_root_or_failed_lookup() {
    let existing = RemoteStorage {
        exists: Ok(true),
        writes: Mutex::default(),
    };
    assert_eq!(
        block_on(create_remote_folder(&existing, "bucket", "photos")),
        Ok(())
    );
    assert_eq!(
        block_on(create_remote_folder(&existing, "bucket", "")),
        Err(ConnectionVerificationError::InvalidRemotePath)
    );
    assert!(existing.writes.lock().unwrap().is_empty());

    let denied = RemoteStorage {
        exists: Err(ProviderError::PermissionDenied),
        writes: Mutex::default(),
    };
    assert_eq!(
        block_on(create_remote_folder(&denied, "bucket", "photos")),
        Err(ConnectionVerificationError::Provider(
            ProviderError::PermissionDenied
        ))
    );
    assert!(denied.writes.lock().unwrap().is_empty());
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = std::task::Waker::noop();
    let mut context = std::task::Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test checker should resolve immediately"),
    }
}
