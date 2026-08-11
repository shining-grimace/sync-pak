use std::{fs, io, path::Path};

/// Removes abandoned files made by SyncPak's atomic writes and archive staging.
///
/// Only direct, regular files with the private `sync-pak-<UUID>.tmp` marker are
/// eligible. Final files, directories, and similarly named user files are untouched.
pub fn remove_stale_files(roots: impl IntoIterator<Item = impl AsRef<Path>>) -> CleanupReport {
    let mut report = CleanupReport::default();
    for root in roots {
        remove_root(root.as_ref(), &mut report);
    }
    report
}

#[derive(Debug, Default)]
pub struct CleanupReport {
    pub removed: usize,
    pub failures: Vec<io::Error>,
}

fn remove_root(root: &Path, report: &mut CleanupReport) {
    let entries = match fs::read_dir(root) {
        Ok(entries) => entries,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return,
        Err(error) => {
            report.failures.push(error);
            return;
        }
    };
    for entry in entries {
        let entry = match entry {
            Ok(entry) => entry,
            Err(error) => {
                report.failures.push(error);
                continue;
            }
        };
        let path = entry.path();
        if !is_sync_pak_temporary(&path) {
            continue;
        }
        match fs::remove_file(path) {
            Ok(()) => report.removed += 1,
            Err(error) => report.failures.push(error),
        }
    }
}

fn is_sync_pak_temporary(path: &Path) -> bool {
    path.is_file()
        && path
            .file_name()
            .and_then(|name| name.to_str())
            .and_then(|name| name.strip_suffix(".tmp"))
            .and_then(|name| name.rsplit_once(".sync-pak-"))
            .is_some_and(|(_, id)| uuid::Uuid::parse_str(id).is_ok())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use super::remove_stale_files;

    #[test]
    fn removes_only_recognised_direct_temporary_files() {
        let root = std::env::temp_dir().join(format!("sync-pak-cleanup-{}", uuid::Uuid::new_v4()));
        fs::create_dir(&root).unwrap();
        let temporary = root.join(format!(".archive.sync-pak-{}.tmp", uuid::Uuid::new_v4()));
        let user_file = root.join(".archive-123.tmp");
        let final_file = root.join("archive.zip");
        let directory = root.join(format!(".folder.sync-pak-{}.tmp", uuid::Uuid::new_v4()));
        fs::write(&temporary, "temporary").unwrap();
        fs::write(&user_file, "user").unwrap();
        fs::write(&final_file, "final").unwrap();
        fs::create_dir(&directory).unwrap();

        let report = remove_stale_files([&root]);

        assert_eq!(report.removed, 1);
        assert!(report.failures.is_empty());
        assert!(!temporary.exists());
        assert!(user_file.exists());
        assert!(final_file.exists());
        assert!(directory.exists());
        fs::remove_dir_all(root).unwrap();
    }
}
