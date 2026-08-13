use crate::{
    inventory::{InventoryEntry, InventoryEntryKind, RelativePath},
    sync_cache::{
        Baseline, CacheNamespace, CacheSnapshot, LocalFingerprint, RemoteIdentity,
        RemoteObservation,
    },
};

use super::cached_source_time;

#[test]
fn a_matching_transfer_baseline_avoids_remote_metadata_lookup() {
    let namespace = CacheNamespace::from_stored("namespace".into());
    let remote = remote();
    let local = local();
    let mut snapshot = CacheSnapshot::default();
    snapshot.insert_baseline(Baseline {
        namespace,
        path: "file.txt".into(),
        local: fingerprint(&local),
        remote: remote.clone(),
        effective_source_unix_seconds: Some(80),
    });

    assert_eq!(
        cached_source_time(
            "bucket",
            "folder/file.txt",
            &RelativePath::new("file.txt").unwrap(),
            &local,
            &remote,
            &snapshot,
        ),
        Some(Some(80))
    );
}

#[test]
fn a_matching_observation_remembers_that_custom_metadata_is_absent() {
    let namespace = CacheNamespace::from_stored("namespace".into());
    let remote = remote();
    let mut snapshot = CacheSnapshot::default();
    snapshot.insert_observation(RemoteObservation {
        namespace,
        bucket: "bucket".into(),
        key: "folder/file.txt".into(),
        identity: remote.clone(),
        source_modified_unix_seconds: None,
    });

    assert_eq!(
        cached_source_time(
            "bucket",
            "folder/file.txt",
            &RelativePath::new("file.txt").unwrap(),
            &local(),
            &remote,
            &snapshot,
        ),
        Some(None)
    );
}

#[test]
fn changed_listing_identity_invalidates_cached_metadata() {
    let namespace = CacheNamespace::from_stored("namespace".into());
    let mut snapshot = CacheSnapshot::default();
    snapshot.insert_observation(RemoteObservation {
        namespace,
        bucket: "bucket".into(),
        key: "folder/file.txt".into(),
        identity: remote(),
        source_modified_unix_seconds: Some(80),
    });
    let changed = RemoteIdentity {
        entity_tag: Some("changed".into()),
        ..remote()
    };

    assert_eq!(
        cached_source_time(
            "bucket",
            "folder/file.txt",
            &RelativePath::new("file.txt").unwrap(),
            &local(),
            &changed,
            &snapshot,
        ),
        None
    );
}

fn local() -> InventoryEntry {
    InventoryEntry::new(
        RelativePath::new("file.txt").unwrap(),
        InventoryEntryKind::File,
        7,
        Some(80),
    )
}

fn fingerprint(entry: &InventoryEntry) -> LocalFingerprint {
    LocalFingerprint {
        kind: entry.kind.clone(),
        byte_size: entry.byte_size,
        modified_unix_seconds: entry.modified_unix_seconds,
    }
}

fn remote() -> RemoteIdentity {
    RemoteIdentity {
        byte_size: 7,
        modified_unix_seconds: Some(100),
        entity_tag: Some("etag".into()),
    }
}
