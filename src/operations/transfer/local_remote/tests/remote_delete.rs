use super::{Provider, block_on, transfer};
use crate::{
    configuration::SyncMode,
    inventory::{Inventory, RelativePath, remote::inventory_from_objects},
    operations::{
        cancellation::CancellationToken,
        retry::RetryPolicy,
        transfer::{modes::mirror::execute_confirmed_mirror, progress::NoopProgressObserver},
    },
    preflight::{
        CaseSensitivity,
        destructive_confirmation::DestructiveConfirmation,
        mirror_selection::{MirrorChoice, MirrorSelection},
        planning::Direction,
        preflight,
    },
    providers::capabilities::{ObjectMetadata, RemoteObject},
};

#[test]
fn confirmed_mirror_deletes_remote_objects_and_their_folder_markers() {
    let remote = inventory_from_objects(
        "sync",
        [
            ("sync/folder/", 0),
            ("sync/folder/image.jpg", 1),
            ("sync/empty folder/", 0),
        ]
        .map(|(key, byte_size)| RemoteObject {
            key: key.into(),
            metadata: ObjectMetadata {
                byte_size,
                modified_unix_seconds: None,
                source_modified_unix_seconds: None,
                content_type: None,
                entity_tag: None,
            },
        }),
    )
    .unwrap();
    let reviewed = preflight(
        SyncMode::Mirror,
        Direction::BothWays,
        &Inventory::default(),
        CaseSensitivity::Sensitive,
        &remote,
        CaseSensitivity::Sensitive,
    )
    .unwrap();
    let mut selection = MirrorSelection::new(reviewed);
    for path in ["folder", "folder/image.jpg", "empty folder"] {
        assert!(selection.choose(
            &RelativePath::new(path).unwrap(),
            Some(MirrorChoice::Delete)
        ));
    }
    let selected = selection.selected().unwrap();
    let confirmation = DestructiveConfirmation::confirm(selected.plan()).unwrap();
    let provider = Provider::default();
    let policy = RetryPolicy::default();

    block_on(execute_confirmed_mirror(
        selected.plan(),
        Some(&confirmation),
        &transfer(&provider, std::path::Path::new("."), &policy),
        &CancellationToken::default(),
        &NoopProgressObserver,
        1,
    ))
    .unwrap();

    assert_eq!(
        provider.deletes.lock().unwrap().as_slice(),
        [
            "sync/folder/image.jpg",
            "sync/empty folder/",
            "sync/folder/"
        ]
    );
}
