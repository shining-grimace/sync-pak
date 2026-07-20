use std::{
    collections::BTreeMap,
    future::Future,
    io::Read,
    path::Path,
    task::{Context, Poll, Waker},
};

use crate::{
    cancellation::CancellationToken,
    inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath},
};

use super::{ArchiveDownloadError, ArchiveDownloader, download_and_create_archive};

struct Downloader {
    files: BTreeMap<String, Vec<u8>>,
    fail: bool,
}

impl ArchiveDownloader for Downloader {
    type Error = &'static str;

    fn download(
        &self,
        source: &RelativePath,
        destination: &Path,
        _: &CancellationToken,
        _: u64,
    ) -> impl Future<Output = Result<(), Self::Error>> {
        let contents = self.files.get(source.as_str()).cloned();
        let destination = destination.to_owned();
        async move {
            if self.fail {
                return Err("provider failed");
            }
            std::fs::create_dir_all(destination.parent().unwrap()).unwrap();
            std::fs::write(destination, contents.unwrap()).unwrap();
            Ok(())
        }
    }
}

#[test]
fn creates_a_local_zip_after_downloading_the_complete_remote_source() {
    let root = temporary_root();
    let destination = root.join("archive.zip");
    let downloader = Downloader {
        files: BTreeMap::from([("folder/file.txt".into(), b"remote contents".to_vec())]),
        fail: false,
    };

    block_on(download_and_create_archive(
        &downloader,
        &inventory(),
        &root,
        &destination,
        &CancellationToken::default(),
        2,
    ))
    .unwrap();

    let mut zip = zip::ZipArchive::new(std::fs::File::open(&destination).unwrap()).unwrap();
    let mut contents = String::new();
    zip.by_name("folder/file.txt")
        .unwrap()
        .read_to_string(&mut contents)
        .unwrap();
    assert_eq!(contents, "remote contents");
    assert!(zip.by_name("empty/").is_ok());
    assert_eq!(staging_trees(&root), 0);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_remote_download_never_creates_an_archive() {
    let root = temporary_root();
    let destination = root.join("archive.zip");
    let downloader = Downloader {
        files: BTreeMap::new(),
        fail: true,
    };

    assert!(matches!(
        block_on(download_and_create_archive(
            &downloader,
            &inventory(),
            &root,
            &destination,
            &CancellationToken::default(),
            2,
        )),
        Err(ArchiveDownloadError::Download("provider failed"))
    ));

    assert!(!destination.exists());
    assert_eq!(staging_trees(&root), 0);
    std::fs::remove_dir_all(root).unwrap();
}

fn inventory() -> Inventory {
    Inventory::new([
        InventoryEntry::new(
            RelativePath::new("empty").unwrap(),
            InventoryEntryKind::Directory,
            0,
            None,
        ),
        InventoryEntry::new(
            RelativePath::new("folder/file.txt").unwrap(),
            InventoryEntryKind::File,
            15,
            None,
        ),
    ])
    .unwrap()
}

fn temporary_root() -> std::path::PathBuf {
    let root = std::env::temp_dir().join(format!("sync-pak-archive-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir(&root).unwrap();
    root
}

fn staging_trees(root: &Path) -> usize {
    std::fs::read_dir(root)
        .unwrap()
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".syncpak-archive-")
        })
        .count()
}

fn block_on<F: Future>(future: F) -> F::Output {
    let waker = Waker::noop();
    let mut context = Context::from_waker(waker);
    let mut future = std::pin::pin!(future);
    match future.as_mut().poll(&mut context) {
        Poll::Ready(output) => output,
        Poll::Pending => panic!("test downloader must not suspend"),
    }
}
