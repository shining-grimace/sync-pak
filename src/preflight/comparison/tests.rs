use super::{EntryStatus, compare};
use crate::inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath};

fn entry(
    path: &str,
    kind: InventoryEntryKind,
    byte_size: u64,
    modified_unix_seconds: Option<i64>,
) -> InventoryEntry {
    InventoryEntry::new(
        RelativePath::new(path).unwrap(),
        kind,
        byte_size,
        modified_unix_seconds,
    )
}

#[test]
fn compares_files_with_a_two_second_timestamp_tolerance() {
    let source = Inventory::new([entry("é.txt", InventoryEntryKind::File, 9, Some(100))]).unwrap();
    let destination =
        Inventory::new([entry("é.txt", InventoryEntryKind::File, 9, Some(102))]).unwrap();

    assert_eq!(
        compare(&source, &destination)[0].status,
        EntryStatus::Unchanged
    );
}

#[test]
fn uses_size_and_available_modification_times() {
    let source = Inventory::new([
        entry("missing-time", InventoryEntryKind::File, 9, None),
        entry("different-size", InventoryEntryKind::File, 9, Some(100)),
        entry("different-time", InventoryEntryKind::File, 9, Some(100)),
    ])
    .unwrap();
    let destination = Inventory::new([
        entry("missing-time", InventoryEntryKind::File, 9, Some(100)),
        entry("different-size", InventoryEntryKind::File, 8, Some(100)),
        entry("different-time", InventoryEntryKind::File, 9, Some(103)),
    ])
    .unwrap();

    let results = compare(&source, &destination);
    assert_eq!(results[0].status, EntryStatus::Changed);
    assert_eq!(results[1].status, EntryStatus::Changed);
    assert_eq!(results[2].status, EntryStatus::Unchanged);
}

#[test]
fn compares_directories_and_symlink_target_text() {
    let source = Inventory::new([
        entry("empty", InventoryEntryKind::Directory, 0, None),
        entry(
            "current",
            InventoryEntryKind::Symlink {
                target: "releases/a".into(),
            },
            0,
            None,
        ),
    ])
    .unwrap();
    let destination = Inventory::new([
        entry("empty", InventoryEntryKind::Directory, 0, Some(100)),
        entry(
            "current",
            InventoryEntryKind::Symlink {
                target: "releases/b".into(),
            },
            0,
            None,
        ),
    ])
    .unwrap();

    let results = compare(&source, &destination);
    assert_eq!(results[0].status, EntryStatus::Changed);
    assert_eq!(results[1].status, EntryStatus::Unchanged);
}

#[test]
fn reports_entries_present_on_only_one_side_as_new() {
    let source =
        Inventory::new([entry("source-only", InventoryEntryKind::File, 1, Some(1))]).unwrap();
    let destination = Inventory::new([entry(
        "destination-only",
        InventoryEntryKind::File,
        1,
        Some(1),
    )])
    .unwrap();

    let results = compare(&source, &destination);
    assert_eq!(results.len(), 2);
    assert!(results.iter().all(|entry| entry.status == EntryStatus::New));
}
