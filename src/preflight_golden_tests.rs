use super::{CaseSensitivity, preflight};
use crate::configuration::SyncMode;
use crate::inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath};
use crate::planning::{Direction, Endpoint, PlannedAction};

fn entry(path: &str, kind: InventoryEntryKind, size: u64, modified: Option<i64>) -> InventoryEntry {
    InventoryEntry::new(RelativePath::new(path).unwrap(), kind, size, modified)
}

fn fixture() -> (Inventory, Inventory) {
    let source = Inventory::new([
        entry(".hidden", InventoryEntryKind::File, 1, Some(10)),
        entry("empty", InventoryEntryKind::Directory, 0, None),
        entry(
            "link",
            InventoryEntryKind::Symlink { target: "a".into() },
            0,
            None,
        ),
        entry("missing-time", InventoryEntryKind::File, 1, None),
        entry("precision", InventoryEntryKind::File, 1, Some(10)),
        entry("source-only", InventoryEntryKind::File, 1, Some(10)),
        entry("type-change", InventoryEntryKind::File, 1, Some(10)),
    ])
    .unwrap();
    let destination = Inventory::new([
        entry(".hidden", InventoryEntryKind::File, 1, Some(10)),
        entry("destination-only", InventoryEntryKind::File, 1, Some(10)),
        entry(
            "link",
            InventoryEntryKind::Symlink { target: "b".into() },
            0,
            None,
        ),
        entry("missing-time", InventoryEntryKind::File, 1, Some(10)),
        entry("precision", InventoryEntryKind::File, 1, Some(12)),
        entry("type-change", InventoryEntryKind::Directory, 0, None),
    ])
    .unwrap();
    (source, destination)
}

#[test]
fn add_only_golden_plan_copies_only_missing_paths_and_warns_for_every_difference() {
    let (source, destination) = fixture();
    let result = preflight(
        SyncMode::AddOnly,
        Direction::Upload,
        &source,
        CaseSensitivity::Sensitive,
        &destination,
        CaseSensitivity::Sensitive,
    )
    .unwrap();

    assert_eq!(
        result.plan().actions(),
        [
            PlannedAction::Copy {
                path: RelativePath::new("empty").unwrap(),
                from: Endpoint::Source,
                to: Endpoint::Destination,
            },
            PlannedAction::SkipChanged {
                path: RelativePath::new("link").unwrap(),
            },
            PlannedAction::SkipChanged {
                path: RelativePath::new("missing-time").unwrap(),
            },
            PlannedAction::Copy {
                path: RelativePath::new("source-only").unwrap(),
                from: Endpoint::Source,
                to: Endpoint::Destination,
            },
            PlannedAction::SkipChanged {
                path: RelativePath::new("type-change").unwrap(),
            },
        ]
    );
}

#[test]
fn mirror_golden_plan_overwrites_differences_then_deletes_destination_only_paths() {
    let (source, destination) = fixture();
    let result = preflight(
        SyncMode::Mirror,
        Direction::Upload,
        &source,
        CaseSensitivity::Sensitive,
        &destination,
        CaseSensitivity::Sensitive,
    )
    .unwrap();

    assert_eq!(
        result.plan().actions(),
        [
            PlannedAction::Delete {
                path: RelativePath::new("destination-only").unwrap(),
                from: Endpoint::Destination,
            },
            PlannedAction::Copy {
                path: RelativePath::new("empty").unwrap(),
                from: Endpoint::Source,
                to: Endpoint::Destination,
            },
            PlannedAction::Overwrite {
                path: RelativePath::new("link").unwrap(),
                from: Endpoint::Source,
                to: Endpoint::Destination,
            },
            PlannedAction::Overwrite {
                path: RelativePath::new("missing-time").unwrap(),
                from: Endpoint::Source,
                to: Endpoint::Destination,
            },
            PlannedAction::Copy {
                path: RelativePath::new("source-only").unwrap(),
                from: Endpoint::Source,
                to: Endpoint::Destination,
            },
            PlannedAction::Overwrite {
                path: RelativePath::new("type-change").unwrap(),
                from: Endpoint::Source,
                to: Endpoint::Destination,
            },
        ]
    );
    assert!(result.plan().requires_confirmation());
}

#[test]
fn large_unicode_inventory_has_a_stable_non_mutating_plan() {
    let source = Inventory::new((0..1_000).map(|number| {
        entry(
            &format!("資料/{number:04}-é.txt"),
            InventoryEntryKind::File,
            number,
            Some(10),
        )
    }))
    .unwrap();
    let destination = Inventory::new((0..500).map(|number| {
        entry(
            &format!("資料/{number:04}-é.txt"),
            InventoryEntryKind::File,
            number,
            Some(11),
        )
    }))
    .unwrap();

    let result = preflight(
        SyncMode::AddOnly,
        Direction::Upload,
        &source,
        CaseSensitivity::Sensitive,
        &destination,
        CaseSensitivity::Sensitive,
    )
    .unwrap();

    assert_eq!(result.comparison().len(), 1_000);
    assert_eq!(result.plan().actions().len(), 500);
    assert!(!result.plan().requires_confirmation());
}
