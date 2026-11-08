use super::*;
use crate::{
    inventory::Inventory,
    preflight::{CaseSensitivity, preflight},
};

mod completeness;

fn inventory(entries: &[(&str, u64)]) -> Inventory {
    Inventory::new(entries.iter().map(|(path, size)| {
        InventoryEntry::new(
            RelativePath::new(*path).unwrap(),
            InventoryEntryKind::File,
            *size,
            Some(10),
        )
    }))
    .unwrap()
}

fn selection(direction: Direction) -> MirrorSelection {
    let local = inventory(&[("local", 1), ("conflict", 2), ("same", 3)]);
    let remote = inventory(&[("remote", 1), ("conflict", 4), ("same", 3)]);
    let (source, destination) = if direction == Direction::Download {
        (&remote, &local)
    } else {
        (&local, &remote)
    };
    MirrorSelection::new(
        preflight(
            SyncMode::Mirror,
            direction,
            source,
            CaseSensitivity::Sensitive,
            destination,
            CaseSensitivity::Sensitive,
        )
        .unwrap(),
    )
}

fn path(path: &str) -> RelativePath {
    RelativePath::new(path).unwrap()
}

#[test]
fn starts_empty_and_undo_restores_original_inventories() {
    let mut selection = selection(Direction::BothWays);
    assert!(selection.selected().is_err());
    assert!(selection.choose(&path("local"), Some(MirrorChoice::Keep)));
    assert_eq!(selection.preview().plan().summary().additions(), 1);
    assert!(selection.choose(&path("local"), None));
    assert!(selection.selected().is_err());
    assert!(selection.preview().plan().actions().is_empty());
    assert!(selection.original().same_inventory_as(&selection.preview()));
    assert!(!selection.choose(&path("same"), Some(MirrorChoice::UseLocal)));
}

#[test]
fn each_direction_strictly_limits_the_side_that_can_change() {
    for direction in [Direction::Upload, Direction::Download] {
        let mut selection = selection(direction);
        let upload = direction == Direction::Upload;
        assert_eq!(
            selection.choose(&path("local"), Some(MirrorChoice::Keep)),
            upload
        );
        assert_eq!(
            selection.choose(&path("remote"), Some(MirrorChoice::Keep)),
            !upload
        );
        assert_eq!(
            selection.choose(&path("local"), Some(MirrorChoice::Delete)),
            !upload
        );
        assert_eq!(
            selection.choose(&path("remote"), Some(MirrorChoice::Delete)),
            upload
        );
        assert_eq!(
            selection.choose(&path("conflict"), Some(MirrorChoice::UseLocal)),
            upload
        );
        assert_eq!(
            selection.choose(&path("conflict"), Some(MirrorChoice::UseRemote)),
            !upload
        );
    }
}

#[test]
fn mixed_plan_binds_confirmation_to_choices() {
    use crate::preflight::destructive_confirmation::DestructiveConfirmation;
    let mut selection = selection(Direction::BothWays);
    selection.choose(&path("local"), Some(MirrorChoice::Keep));
    selection.choose(&path("remote"), Some(MirrorChoice::Keep));
    selection.choose(&path("conflict"), Some(MirrorChoice::UseRemote));
    let reviewed = selection.selected().unwrap();
    assert_eq!(reviewed.plan().summary().additions(), 2);
    assert_eq!(reviewed.plan().summary().overwrites(), 1);
    let confirmation = DestructiveConfirmation::confirm(reviewed.plan()).unwrap();
    selection.choose(&path("conflict"), Some(MirrorChoice::UseLocal));
    assert!(
        confirmation
            .verify(selection.selected().unwrap().plan())
            .is_err()
    );
}

#[test]
fn folder_deletion_requires_explicit_descendant_deletion() {
    let source = Inventory::default();
    let mut entries = inventory(&[("folder/child", 1)])
        .entries()
        .cloned()
        .collect::<Vec<_>>();
    entries.push(InventoryEntry::new(
        path("folder"),
        InventoryEntryKind::Directory,
        0,
        None,
    ));
    let destination = Inventory::new(entries).unwrap();
    let original = preflight(
        SyncMode::Mirror,
        Direction::BothWays,
        &source,
        CaseSensitivity::Sensitive,
        &destination,
        CaseSensitivity::Sensitive,
    )
    .unwrap();
    let mut selection = MirrorSelection::new(original);
    selection.choose(&path("folder"), Some(MirrorChoice::Delete));
    assert!(selection.selected().is_err());
    selection.choose(&path("folder/child"), Some(MirrorChoice::Keep));
    assert!(selection.selected().is_err());
    selection.choose(&path("folder/child"), Some(MirrorChoice::Delete));
    assert_eq!(
        selection.selected().unwrap().plan().summary().deletions(),
        2
    );
}

#[test]
fn additive_copies_start_selected_and_undo_requires_a_new_choice() {
    let original = preflight(
        SyncMode::AddOnly,
        Direction::BothWays,
        &inventory(&[("local", 1)]),
        CaseSensitivity::Sensitive,
        &inventory(&[("remote", 1)]),
        CaseSensitivity::Sensitive,
    )
    .unwrap();
    let mut selection = MirrorSelection::new(original);
    assert_eq!(
        selection.selected().unwrap().plan().summary().additions(),
        2
    );
    assert!(selection.choose(&path("local"), None));
    assert!(selection.selected().is_err());
    assert!(!selection.choose(&path("remote"), Some(MirrorChoice::Delete)));
    assert!(selection.choose(&path("local"), Some(MirrorChoice::Keep)));
    assert_eq!(
        selection.selected().unwrap().plan().mode(),
        SyncMode::AddOnly
    );
}
