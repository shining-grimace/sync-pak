use super::*;

#[test]
fn portable_paths_reject_absolute_and_traversing_paths() {
    for path in [
        "/photos",
        "../photos",
        "a/../b",
        "C:/Photos",
        "a\\b",
        "a//b",
        ".",
        "a/",
        "a\0b",
    ] {
        assert!(relative_path(path).is_err(), "{path:?}");
    }
    for path in ["", "photos", "photos/Family Holiday", "日本語/100%#"] {
        relative_path(path).unwrap();
    }
    assert!(local_path("relative-folder", "photos").is_err());
}

#[test]
fn android_descendants_keep_the_tree_grant_separate_from_relative_names() {
    let tree = "content://com.android.externalstorage.documents/tree/primary%3ADocuments";
    assert_eq!(local_path(tree, "").unwrap(), tree);
    assert_eq!(
        local_path(tree, "Albums/100%#").unwrap(),
        format!("{tree}#Albums/100%#")
    );
    assert!(local_path(tree, "../Secret").is_err());
}

#[cfg(unix)]
#[test]
fn symlinks_cannot_escape_a_bound_local_root() {
    let base = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&base).unwrap();
    std::os::unix::fs::symlink(std::env::temp_dir(), base.join("outside")).unwrap();
    assert!(local_path(base.to_str().unwrap(), "outside").is_err());
    assert!(local_path(base.to_str().unwrap(), "outside/not-yet-created").is_err());
    std::fs::remove_dir_all(base).unwrap();
}
