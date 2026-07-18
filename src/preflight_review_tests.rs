use super::{ReviewStatus, review_items};
use crate::configuration::SyncMode;
use crate::inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath};
use crate::planning::Direction;
use crate::preflight::{CaseSensitivity, preflight};

fn file(path: &str, size: u64) -> InventoryEntry {
    InventoryEntry::new(
        RelativePath::new(path).unwrap(),
        InventoryEntryKind::File,
        size,
        Some(1),
    )
}

#[test]
fn review_uses_user_facing_action_terms_and_hides_add_only_destination_only_files() {
    let source = Inventory::new([file("changed", 2), file("source-only", 1)]).unwrap();
    let destination = Inventory::new([file("changed", 1), file("destination-only", 1)]).unwrap();
    let preflight = preflight(
        SyncMode::AddOnly,
        Direction::Upload,
        &source,
        CaseSensitivity::Sensitive,
        &destination,
        CaseSensitivity::Sensitive,
    )
    .unwrap();

    assert_eq!(
        review_items(&preflight),
        [
            super::ReviewItem {
                path: RelativePath::new("changed").unwrap(),
                status: ReviewStatus::Warning,
            },
            super::ReviewItem {
                path: RelativePath::new("source-only").unwrap(),
                status: ReviewStatus::New,
            },
        ]
    );
}

#[test]
fn mirror_review_marks_destructive_actions() {
    let source = Inventory::new([file("changed", 2)]).unwrap();
    let destination = Inventory::new([file("changed", 1), file("destination-only", 1)]).unwrap();
    let preflight = preflight(
        SyncMode::Mirror,
        Direction::Upload,
        &source,
        CaseSensitivity::Sensitive,
        &destination,
        CaseSensitivity::Sensitive,
    )
    .unwrap();

    let statuses = review_items(&preflight)
        .into_iter()
        .map(|item| item.status)
        .collect::<Vec<_>>();
    assert_eq!(
        statuses,
        [ReviewStatus::WillOverwrite, ReviewStatus::WillDelete]
    );
}
