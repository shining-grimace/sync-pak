use uuid::Uuid;

use crate::inventory::InventoryEntryKind;

use super::{
    Baseline, CacheNamespace, LocalFingerprint, RemoteIdentity, RemoteObservation, SyncCache,
};

#[test]
fn persists_remote_observations_and_successful_transfer_baselines() {
    let directory = temporary_directory();
    let cache = SyncCache::for_configuration(&directory.join("config.json")).unwrap();
    let namespace = CacheNamespace::from_stored("test-namespace".into());
    let remote = remote_identity();
    let observation = RemoteObservation {
        namespace: namespace.clone(),
        bucket: "bucket".into(),
        key: "folder/file.txt".into(),
        identity: remote.clone(),
        source_modified_unix_seconds: Some(90),
    };
    let baseline = Baseline {
        namespace: namespace.clone(),
        path: "file.txt".into(),
        local: LocalFingerprint {
            kind: InventoryEntryKind::File,
            byte_size: 7,
            modified_unix_seconds: Some(90),
        },
        remote,
        effective_source_unix_seconds: Some(90),
    };

    cache.record_transfer(&observation, &baseline);

    let snapshot = cache.snapshot(&namespace);
    assert_eq!(
        snapshot.observation("bucket", "folder/file.txt").unwrap(),
        &observation
    );
    assert_eq!(snapshot.baseline("file.txt").unwrap(), &baseline);
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn replaces_a_corrupt_database_and_continues_with_an_empty_cache() {
    let directory = temporary_directory();
    std::fs::write(directory.join("sync-metadata.sqlite3"), b"not sqlite").unwrap();

    let cache = SyncCache::for_configuration(&directory.join("config.json")).unwrap();
    let namespace = CacheNamespace::from_stored("test-namespace".into());

    assert!(cache.snapshot(&namespace).baseline("file.txt").is_none());
    assert!(std::fs::read_dir(&directory).unwrap().any(|entry| {
        entry
            .unwrap()
            .file_name()
            .to_string_lossy()
            .starts_with("sync-metadata.corrupt-")
    }));
    std::fs::remove_dir_all(directory).unwrap();
}

#[test]
fn refuses_to_reuse_remote_identities_without_listing_change_tokens() {
    let mut identity = remote_identity();
    assert!(identity.is_cacheable());
    identity.entity_tag = None;
    assert!(!identity.is_cacheable());
    identity.entity_tag = Some("etag".into());
    identity.modified_unix_seconds = None;
    assert!(!identity.is_cacheable());
}

#[test]
fn invalidation_removes_both_halves_of_a_synchronization_record() {
    let directory = temporary_directory();
    let cache = SyncCache::for_configuration(&directory.join("config.json")).unwrap();
    let namespace = CacheNamespace::from_stored("test-namespace".into());
    let remote = remote_identity();
    let observation = RemoteObservation {
        namespace: namespace.clone(),
        bucket: "bucket".into(),
        key: "folder/file.txt".into(),
        identity: remote.clone(),
        source_modified_unix_seconds: Some(90),
    };
    let baseline = Baseline {
        namespace: namespace.clone(),
        path: "file.txt".into(),
        local: LocalFingerprint {
            kind: InventoryEntryKind::File,
            byte_size: 7,
            modified_unix_seconds: Some(90),
        },
        remote,
        effective_source_unix_seconds: Some(90),
    };
    cache.record_transfer(&observation, &baseline);

    cache.invalidate(&namespace, "bucket", "folder/file.txt", "file.txt");

    let snapshot = cache.snapshot(&namespace);
    assert!(snapshot.observation("bucket", "folder/file.txt").is_none());
    assert!(snapshot.baseline("file.txt").is_none());
    std::fs::remove_dir_all(directory).unwrap();
}

fn remote_identity() -> RemoteIdentity {
    RemoteIdentity {
        byte_size: 7,
        modified_unix_seconds: Some(100),
        entity_tag: Some("etag".into()),
    }
}

fn temporary_directory() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("sync-pak-cache-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&path).unwrap();
    path
}
