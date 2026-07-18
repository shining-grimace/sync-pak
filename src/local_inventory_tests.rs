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

#[cfg(unix)]
#[test]
fn rejects_non_utf8_filesystem_names_without_lossy_conversion() {
    use std::ffi::OsString;
    use std::os::unix::ffi::OsStringExt;

    let root = temporary_directory();
    let name = OsString::from_vec(b"not-utf8-\xff".to_vec());
    fs::write(root.join(name), "contents").unwrap();

    let error = NativeLocalInventory.inventory(&root).unwrap_err();
    fs::remove_dir_all(&root).unwrap();

    assert!(matches!(error, super::LocalInventoryError::NonUtf8Path(_)));
}
