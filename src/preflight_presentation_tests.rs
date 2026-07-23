use super::PreflightPresentation;
use crate::{
    configuration::SyncMode,
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    planning::Direction,
    preflight::{CaseSensitivity, preflight},
};

fn file(path: &str, size: u64) -> InventoryEntry {
    InventoryEntry::new(
        RelativePath::new(path).unwrap(),
        InventoryEntryKind::File,
        size,
        Some(1),
    )
}

#[test]
fn presents_mirror_counts_destructive_confirmation_and_item_labels() {
    let source = Inventory::new([file("changed", 2), file("new", 1)]).unwrap();
    let destination = Inventory::new([file("changed", 1), file("deleted", 1)]).unwrap();
    let preflight = preflight(
        SyncMode::Mirror,
        Direction::Upload,
        &source,
        CaseSensitivity::Sensitive,
        &destination,
        CaseSensitivity::Sensitive,
    )
    .unwrap();

    let presentation = PreflightPresentation::from(&preflight);

    assert_eq!(presentation.additions, "1 new · 1 byte");
    assert_eq!(presentation.overwrites, "1 overwrite · 2 bytes");
    assert_eq!(presentation.deletions, "1 deletion · 1 byte");
    assert_eq!(presentation.skipped, "0 skipped");
    assert_eq!(presentation.start_action, "Start mirror");
    assert!(presentation.requires_mirror_confirmation);
    assert_eq!(
        presentation
            .items
            .iter()
            .find(|item| item.path == "changed")
            .unwrap()
            .status,
        "Will overwrite"
    );
    assert_eq!(
        presentation
            .items
            .iter()
            .find(|item| item.path == "changed")
            .unwrap()
            .detail,
        "Source: file · 2 bytes · Destination: file · 1 byte"
    );
    assert_eq!(
        presentation
            .items
            .iter()
            .find(|item| item.path == "deleted")
            .unwrap()
            .status,
        "Will delete"
    );
}
