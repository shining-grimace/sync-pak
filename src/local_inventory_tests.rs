use std::fs;
use std::path::Path;

use uuid::Uuid;

use super::{LocalInventoryAccess, NativeLocalInventory};
use crate::inventory::InventoryEntryKind;

fn temporary_directory() -> std::path::PathBuf {
    let path = std::env::temp_dir().join(format!("sync-pak-inventory-{}", Uuid::new_v4()));
    fs::create_dir(&path).unwrap();
    path
}

#[test]
fn inventories_hidden_files_empty_directories_and_symlinks_without_following_them() {
    let root = temporary_directory();
    fs::write(root.join(".hidden"), "contents").unwrap();
    fs::create_dir(root.join("empty")).unwrap();
    fs::create_dir(root.join("files")).unwrap();
    fs::write(root.join("files/é.txt"), "contents").unwrap();
    #[cfg(unix)]
    std::os::unix::fs::symlink("files/é.txt", root.join("current")).unwrap();

    let inventory = NativeLocalInventory.inventory(&root).unwrap();
    fs::remove_dir_all(&root).unwrap();
    let paths = inventory
        .entries()
        .map(|entry| (entry.path.as_str(), &entry.kind))
        .collect::<Vec<_>>();

    assert!(paths.contains(&(".hidden", &InventoryEntryKind::File)));
    assert!(paths.contains(&("empty", &InventoryEntryKind::Directory)));
    assert!(paths.contains(&("files/é.txt", &InventoryEntryKind::File)));
    #[cfg(unix)]
    assert!(paths.contains(&(
        "current",
        &InventoryEntryKind::Symlink {
            target: "files/é.txt".into()
        }
    )));
}

#[test]
fn reports_a_missing_root_with_the_filesystem_error() {
    let missing = Path::new("/definitely-not-a-sync-pak-folder");
    let error = NativeLocalInventory.inventory(missing).unwrap_err();
    assert!(error.to_string().contains("read directory"));
}
