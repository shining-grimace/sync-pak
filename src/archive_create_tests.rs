use std::io::Read;

use crate::{
    cancellation::CancellationToken,
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
    transfer_paths::LocalTransferRoot,
};

use super::{ArchiveCreateError, create_archive, stage_archive, stage_archive_with_cancellation};

fn entry(path: &str, kind: InventoryEntryKind) -> InventoryEntry {
    InventoryEntry::new(RelativePath::new(path).unwrap(), kind, 0, None)
}

#[test]
fn creates_a_zip_with_files_and_empty_directories_without_replacing_existing_archives() {
    let root = std::env::temp_dir().join(format!("sync-pak-archive-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("file.txt"), "contents").unwrap();
    let destination = root.join("archive.zip");
    let inventory = Inventory::new([
        entry("file.txt", InventoryEntryKind::File),
        entry("empty", InventoryEntryKind::Directory),
    ])
    .unwrap();

    create_archive(&LocalTransferRoot::new(&root), &inventory, &destination).unwrap();

    let file = std::fs::File::open(&destination).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut contents = String::new();
    archive
        .by_name("file.txt")
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    assert_eq!(contents, "contents");
    assert!(archive.by_name("empty/").is_ok());
    assert!(matches!(
        create_archive(&LocalTransferRoot::new(&root), &inventory, &destination),
        Err(ArchiveCreateError::Collision)
    ));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn staging_keeps_a_complete_archive_local_until_the_caller_discards_it() {
    let root = std::env::temp_dir().join(format!("sync-pak-archive-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("file.txt"), "contents").unwrap();
    let inventory = Inventory::new([entry("file.txt", InventoryEntryKind::File)]).unwrap();

    let staged = stage_archive(
        &LocalTransferRoot::new(&root),
        &inventory,
        &root,
        "archive.zip".as_ref(),
    )
    .unwrap();

    assert!(staged.path().exists());
    assert!(!root.join("archive.zip").exists());
    staged.discard().unwrap();
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn cancelled_staging_removes_its_partial_archive() {
    let root = std::env::temp_dir().join(format!("sync-pak-archive-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    std::fs::write(root.join("file.txt"), "contents").unwrap();
    let cancellation = CancellationToken::default();
    cancellation.cancel();

    assert!(matches!(
        stage_archive_with_cancellation(
            &LocalTransferRoot::new(&root),
            &Inventory::new([entry("file.txt", InventoryEntryKind::File)]).unwrap(),
            &root,
            "archive.zip".as_ref(),
            &cancellation,
        ),
        Err(ArchiveCreateError::Cancelled)
    ));

    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    std::fs::remove_dir_all(&root).unwrap();
}
