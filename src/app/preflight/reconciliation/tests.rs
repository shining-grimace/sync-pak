use super::*;
use crate::{
    configuration::SyncMode,
    inventory::{Inventory, InventoryEntryKind, RelativePath},
    preflight::{CaseSensitivity, planning::Direction, preflight},
};

fn entry(path: &str, size: u64) -> InventoryEntry {
    InventoryEntry::new(
        RelativePath::new(path).unwrap(),
        InventoryEntryKind::File,
        size,
        Some(10),
    )
}

#[test]
fn selection_projects_both_sides_without_reclassifying_and_undo_restores_missing_copy() {
    let local = Inventory::new([entry("local", 1), entry("conflict", 2)]).unwrap();
    let remote = Inventory::new([entry("conflict", 4)]).unwrap();
    let original = preflight(
        SyncMode::Mirror,
        Direction::BothWays,
        &local,
        CaseSensitivity::Sensitive,
        &remote,
        CaseSensitivity::Sensitive,
    )
    .unwrap();
    let mut selection = MirrorSelection::new(original);
    let [isolated, conflicts, _] = rows(&selection, true);
    assert_eq!(isolated[0].remote_detail, "Not present");
    assert!(!isolated[0].selected);
    assert!(!conflicts[0].selected);
    selection.choose(
        &RelativePath::new("local").unwrap(),
        Some(MirrorChoice::Keep),
    );
    selection.choose(
        &RelativePath::new("conflict").unwrap(),
        Some(MirrorChoice::UseRemote),
    );
    let [isolated, conflicts, _] = rows(&selection, true);
    assert_eq!(isolated.len(), 1);
    assert_eq!(conflicts.len(), 1);
    assert!(isolated[0].remote_detail.contains("Will copy from local"));
    assert!(isolated[0].local_changed && isolated[0].remote_changed);
    assert!(conflicts[0].local_detail.contains("4 bytes"));
    assert!(
        conflicts[0]
            .local_detail
            .contains("Will overwrite with remote")
    );
    selection.choose(&RelativePath::new("local").unwrap(), None);
    let [isolated, _, _] = rows(&selection, true);
    assert_eq!(isolated[0].remote_detail, "Not present");
    assert!(!isolated[0].selected);
}

#[test]
fn add_only_preselects_permitted_copies_and_leaves_conflicts_untouched() {
    let local = Inventory::new([entry("local", 1), entry("conflict", 2)]).unwrap();
    let remote = Inventory::new([entry("remote", 1), entry("conflict", 4)]).unwrap();
    let original = preflight(
        SyncMode::AddOnly,
        Direction::Upload,
        &local,
        CaseSensitivity::Sensitive,
        &remote,
        CaseSensitivity::Sensitive,
    )
    .unwrap();
    let selection = MirrorSelection::new(original);
    let [isolated, conflicts, _] = rows(&selection, false);
    assert!(
        isolated
            .iter()
            .find(|row| row.path == "local")
            .unwrap()
            .selected
    );
    assert!(
        !isolated
            .iter()
            .find(|row| row.path == "remote")
            .unwrap()
            .selected
    );
    assert_eq!(conflicts[0].action_label, "Skipped");
    assert!(!conflicts[0].selected);
}
