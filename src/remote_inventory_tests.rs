use super::{RemoteInventoryError, inventory_from_objects};
use crate::inventory::InventoryEntryKind;
use crate::provider_capabilities::{ObjectMetadata, RemoteObject};

fn object(key: &str, byte_size: u64) -> RemoteObject {
    RemoteObject {
        key: key.into(),
        metadata: ObjectMetadata {
            byte_size,
            modified_unix_seconds: Some(10),
            content_type: None,
            entity_tag: None,
        },
    }
}

#[test]
fn converts_remote_prefix_objects_and_directory_markers_to_an_inventory() {
    let inventory = inventory_from_objects(
        "backup",
        [
            object("backup/.hidden", 1),
            object("backup/empty/", 0),
            object("backup/folder/é.txt", 2),
        ],
    )
    .unwrap();

    let entries = inventory.entries().collect::<Vec<_>>();
    assert_eq!(entries.len(), 4);
    assert!(entries.iter().any(
        |entry| entry.path.as_str() == "folder" && entry.kind == InventoryEntryKind::Directory
    ));
    assert!(
        entries
            .iter()
            .any(|entry| entry.path.as_str() == "empty"
                && entry.kind == InventoryEntryKind::Directory)
    );
    assert!(
        entries
            .iter()
            .any(|entry| entry.path.as_str() == "folder/é.txt")
    );
}

#[test]
fn rejects_keys_outside_the_prefix_and_file_directory_conflicts() {
    assert!(matches!(
        inventory_from_objects("backup", [object("other/file", 1)]),
        Err(RemoteInventoryError::ObjectOutsidePrefix(_))
    ));
    assert!(matches!(
        inventory_from_objects("", [object("folder", 1), object("folder/file", 1)]),
        Err(RemoteInventoryError::ConflictingEntry(_))
    ));
}
