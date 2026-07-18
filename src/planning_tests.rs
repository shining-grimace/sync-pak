use super::{Direction, Endpoint, PlanError, PlannedAction, plan};
use crate::comparison::compare;
use crate::configuration::SyncMode;
use crate::inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath};

fn file(path: &str, size: u64, modified: Option<i64>) -> InventoryEntry {
    InventoryEntry::new(
        RelativePath::new(path).unwrap(),
        InventoryEntryKind::File,
        size,
        modified,
    )
}

fn comparison() -> Vec<crate::comparison::ComparedEntry> {
    let source =
        Inventory::new([file("changed", 2, Some(1)), file("source-only", 1, Some(1))]).unwrap();
    let destination = Inventory::new([
        file("changed", 1, Some(1)),
        file("destination-only", 1, Some(1)),
    ])
    .unwrap();
    compare(&source, &destination)
}

#[test]
fn add_only_copies_only_missing_source_paths_and_skips_changed_paths() {
    let plan = plan(SyncMode::AddOnly, Direction::Upload, &comparison()).unwrap();

    assert_eq!(
        plan.actions(),
        [
            PlannedAction::SkipChanged {
                path: RelativePath::new("changed").unwrap(),
            },
            PlannedAction::Copy {
                path: RelativePath::new("source-only").unwrap(),
                from: Endpoint::Source,
                to: Endpoint::Destination,
            },
        ]
    );
    assert!(!plan.requires_confirmation());
}

#[test]
fn additive_both_ways_copies_each_unique_path_without_overwriting() {
    let plan = plan(SyncMode::AddOnly, Direction::BothWays, &comparison()).unwrap();

    assert!(plan.actions().contains(&PlannedAction::Copy {
        path: RelativePath::new("destination-only").unwrap(),
        from: Endpoint::Destination,
        to: Endpoint::Source,
    }));
    assert_eq!(plan.actions().len(), 3);
}

#[test]
fn mirror_plans_overwrites_and_deletes_for_confirmation() {
    let plan = plan(SyncMode::Mirror, Direction::Upload, &comparison()).unwrap();

    assert!(plan.actions().contains(&PlannedAction::Delete {
        path: RelativePath::new("destination-only").unwrap(),
        from: Endpoint::Destination,
    }));
    assert!(plan.actions().iter().any(PlannedAction::is_destructive));
    assert!(plan.requires_confirmation());
}

#[test]
fn archive_has_a_non_mutating_creation_preview_and_rejects_both_ways() {
    let archive = plan(SyncMode::Archive, Direction::Download, &comparison()).unwrap();
    assert_eq!(
        archive.actions(),
        [PlannedAction::CreateArchive {
            from: Endpoint::Source,
            to: Endpoint::Destination,
        }]
    );
    assert_eq!(
        plan(SyncMode::Mirror, Direction::BothWays, &comparison()),
        Err(PlanError::BothWaysRequiresAddOnly)
    );
}
