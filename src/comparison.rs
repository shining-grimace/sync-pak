use crate::inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath};

const MODIFICATION_TIME_TOLERANCE_SECONDS: u64 = 2;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryStatus {
    New,
    Unchanged,
    Changed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComparedEntry {
    pub path: RelativePath,
    pub status: EntryStatus,
    pub source: Option<InventoryEntry>,
    pub destination: Option<InventoryEntry>,
}

/// Compares two read-only inventories without opening any file contents.
pub fn compare(source: &Inventory, destination: &Inventory) -> Vec<ComparedEntry> {
    let mut entries = Vec::new();
    for source_entry in source.entries() {
        let destination_entry = destination.get(&source_entry.path);
        entries.push(ComparedEntry {
            path: source_entry.path.clone(),
            status: destination_entry.map_or(EntryStatus::New, |entry| {
                status_for_matching_paths(source_entry, entry)
            }),
            source: Some(source_entry.clone()),
            destination: destination_entry.cloned(),
        });
    }
    for destination_entry in destination.entries() {
        if source.get(&destination_entry.path).is_none() {
            entries.push(ComparedEntry {
                path: destination_entry.path.clone(),
                status: EntryStatus::New,
                source: None,
                destination: Some(destination_entry.clone()),
            });
        }
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    entries
}

fn status_for_matching_paths(source: &InventoryEntry, destination: &InventoryEntry) -> EntryStatus {
    let unchanged = match (&source.kind, &destination.kind) {
        (InventoryEntryKind::Directory, InventoryEntryKind::Directory) => true,
        (
            InventoryEntryKind::Symlink { target: left },
            InventoryEntryKind::Symlink { target: right },
        ) => left == right,
        (InventoryEntryKind::File, InventoryEntryKind::File) => {
            source.byte_size == destination.byte_size
                && modification_times_match(
                    source.modified_unix_seconds,
                    destination.modified_unix_seconds,
                )
        }
        _ => false,
    };
    if unchanged {
        EntryStatus::Unchanged
    } else {
        EntryStatus::Changed
    }
}

fn modification_times_match(source: Option<i64>, destination: Option<i64>) -> bool {
    match (source, destination) {
        (Some(source), Some(destination)) => {
            source.abs_diff(destination) <= MODIFICATION_TIME_TOLERANCE_SECONDS
        }
        _ => false,
    }
}

#[cfg(test)]
mod tests {
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
        let source =
            Inventory::new([entry("é.txt", InventoryEntryKind::File, 9, Some(100))]).unwrap();
        let destination =
            Inventory::new([entry("é.txt", InventoryEntryKind::File, 9, Some(102))]).unwrap();

        assert_eq!(
            compare(&source, &destination)[0].status,
            EntryStatus::Unchanged
        );
    }

    #[test]
    fn marks_files_changed_when_metadata_is_missing_or_different() {
        let source = Inventory::new([
            entry("missing-time", InventoryEntryKind::File, 9, None),
            entry("different-size", InventoryEntryKind::File, 9, Some(100)),
        ])
        .unwrap();
        let destination = Inventory::new([
            entry("missing-time", InventoryEntryKind::File, 9, Some(100)),
            entry("different-size", InventoryEntryKind::File, 8, Some(100)),
        ])
        .unwrap();

        assert!(
            compare(&source, &destination)
                .iter()
                .all(|entry| entry.status == EntryStatus::Changed)
        );
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
}
