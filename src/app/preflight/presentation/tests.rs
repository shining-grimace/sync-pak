use super::PreflightPresentation;
use crate::{
    configuration::SyncMode,
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    preflight::planning::Direction,
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
    assert_eq!(presentation.unchanged, "0 unchanged");
    assert_eq!(presentation.planned_file_changes, 3);
    assert!(presentation.has_planned_actions);
    assert_eq!(presentation.start_action, "Start Mirror");
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
        "Source: file · 2 bytes · Destination: file · 1 byte · Sizes differ"
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

#[test]
fn presents_unchanged_files_without_a_start_action() {
    let source = Inventory::new([file("unchanged", 1)]).unwrap();
    let destination = Inventory::new([file("unchanged", 1)]).unwrap();
    let preflight = preflight(
        SyncMode::AddOnly,
        Direction::Upload,
        &source,
        CaseSensitivity::Sensitive,
        &destination,
        CaseSensitivity::Sensitive,
    )
    .unwrap();

    let presentation = PreflightPresentation::from(&preflight);

    assert_eq!(presentation.unchanged, "1 unchanged");
    assert_eq!(presentation.planned_file_changes, 0);
    assert!(!presentation.has_planned_actions);
    assert_eq!(presentation.start_action, "No Action Needed");
}

#[test]
fn folders_never_display_byte_sizes_or_size_comparison_reasons() {
    let folder = |path, size| {
        InventoryEntry::new(
            RelativePath::new(path).unwrap(),
            InventoryEntryKind::Directory,
            size,
            Some(1),
        )
    };
    let source = Inventory::new([folder("existing", 4096), folder("new", 4096)]).unwrap();
    let destination = Inventory::new([folder("existing", 0), folder("deleted", 0)]).unwrap();
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
    let detail = |path| {
        presentation
            .items
            .iter()
            .find(|item| item.path == path)
            .unwrap()
            .detail
            .as_str()
    };
    assert_eq!(detail("existing"), "Source: folder · Destination: folder");
    assert_eq!(detail("new"), "Source: folder");
    assert_eq!(detail("deleted"), "Destination: folder");
}

#[test]
fn folder_file_conflicts_report_types_instead_of_comparing_sizes() {
    use crate::preflight::review::{ReviewEntryDetails, ReviewItem, ReviewStatus};
    let folder = ReviewEntryDetails {
        kind: InventoryEntryKind::Directory,
        byte_size: 4096,
        modified_unix_seconds: Some(1),
    };
    let file = ReviewEntryDetails {
        kind: InventoryEntryKind::File,
        byte_size: 12,
        modified_unix_seconds: Some(1),
    };
    for (source, destination) in [(folder.clone(), file.clone()), (file, folder)] {
        let item = super::PreflightItemPresentation::from(ReviewItem {
            path: RelativePath::new("conflict").unwrap(),
            status: ReviewStatus::Changed,
            source: Some(source),
            destination: Some(destination),
        });
        assert!(item.detail.ends_with("Entry types differ"));
        assert!(!item.detail.contains("Sizes differ"));
        assert!(!item.detail.contains("4096"));
        assert!(item.detail.contains("file · 12 bytes"));
    }
}
