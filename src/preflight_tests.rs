use super::{CaseSensitivity, PreflightError, preflight};
use crate::configuration::SyncMode;
use crate::inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath};
use crate::planning::{Direction, Endpoint};

fn inventory(paths: &[&str]) -> Inventory {
    Inventory::new(paths.iter().map(|path| {
        InventoryEntry::new(
            RelativePath::new(*path).unwrap(),
            InventoryEntryKind::File,
            1,
            Some(1),
        )
    }))
    .unwrap()
}

#[test]
fn rejects_source_case_collisions_for_a_case_insensitive_destination() {
    let error = preflight(
        SyncMode::AddOnly,
        Direction::Upload,
        &inventory(&["Readme", "README"]),
        CaseSensitivity::Sensitive,
        &inventory(&[]),
        CaseSensitivity::Insensitive,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        PreflightError::CaseCollision {
            endpoint: Endpoint::Source,
            ..
        }
    ));
}

#[test]
fn allows_case_collisions_when_the_destination_is_case_sensitive() {
    let result = preflight(
        SyncMode::AddOnly,
        Direction::Upload,
        &inventory(&["Readme", "README"]),
        CaseSensitivity::Sensitive,
        &inventory(&[]),
        CaseSensitivity::Sensitive,
    )
    .unwrap();

    assert_eq!(result.plan().actions().len(), 2);
}

#[test]
fn validates_both_directions_when_additive_both_ways_targets_are_insensitive() {
    let error = preflight(
        SyncMode::AddOnly,
        Direction::BothWays,
        &inventory(&[]),
        CaseSensitivity::Insensitive,
        &inventory(&["Readme", "README"]),
        CaseSensitivity::Sensitive,
    )
    .unwrap_err();

    assert!(matches!(
        error,
        PreflightError::CaseCollision {
            endpoint: Endpoint::Destination,
            ..
        }
    ));
}

#[test]
fn archive_preflight_preserves_case_sensitive_zip_entries() {
    let result = preflight(
        SyncMode::Archive,
        Direction::Upload,
        &inventory(&["Readme", "README"]),
        CaseSensitivity::Sensitive,
        &inventory(&[]),
        CaseSensitivity::Insensitive,
    )
    .unwrap();

    assert_eq!(result.comparison().len(), 2);
}

#[test]
fn becomes_stale_when_either_inventory_metadata_changes() {
    let source = inventory(&["source"]);
    let destination = inventory(&[]);
    let result = preflight(
        SyncMode::Mirror,
        Direction::Upload,
        &source,
        CaseSensitivity::Sensitive,
        &destination,
        CaseSensitivity::Sensitive,
    )
    .unwrap();

    assert!(result.is_current(&source, &destination));
    assert!(!result.is_current(&inventory(&["source", "new"]), &destination));
}
