use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, UNIX_EPOCH},
};

#[cfg(target_os = "android")]
use std::io::Write;

use crate::{
    inventory::local::{LocalInventoryAccess, LocalInventoryError, NativeLocalInventory},
    inventory::{Inventory, RelativePath},
    operations::transfer::local_remote::root_metadata::{
        LocalEntryMetadata, native_metadata, verify_directory,
    },
    platform::atomic_write::atomic_write,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum LocalTransferRoot {
    FileSystem(PathBuf),
    #[cfg(target_os = "android")]
    AndroidTree(String),
}

impl LocalTransferRoot {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self::FileSystem(root.into())
    }

    pub fn from_config(root: &str) -> Self {
        #[cfg(target_os = "android")]
        if root.starts_with("content://") {
            return Self::AndroidTree(root.into());
        }
        Self::new(root)
    }

    pub fn verify(&self) -> io::Result<()> {
        match self {
            Self::FileSystem(root) => verify_directory(root),
            #[cfg(target_os = "android")]
            Self::AndroidTree(uri) => crate::platform::android::document_tree::access::verify(uri),
        }
    }

    pub fn inventory(&self) -> Result<Inventory, LocalInventoryError> {
        match self {
            Self::FileSystem(root) => NativeLocalInventory.inventory(root),
            #[cfg(target_os = "android")]
            Self::AndroidTree(uri) => {
                crate::platform::android::document_tree::access::inventory(uri)
                    .map_err(LocalInventoryError::Platform)
            }
        }
    }

    pub fn metadata(&self, relative: &RelativePath) -> io::Result<LocalEntryMetadata> {
        match self {
            Self::FileSystem(root) => native_metadata(&resolve(root, relative)),
            #[cfg(target_os = "android")]
            Self::AndroidTree(uri) => {
                let metadata =
                    crate::platform::android::document_tree::access::metadata(uri, relative)?;
                Ok(LocalEntryMetadata {
                    byte_size: metadata.size,
                    modified_unix_seconds: metadata.modified,
                    kind: if metadata.is_directory() {
                        crate::inventory::InventoryEntryKind::Directory
                    } else {
                        crate::inventory::InventoryEntryKind::File
                    },
                })
            }
        }
    }

    pub fn open_read(&self, relative: &RelativePath) -> io::Result<fs::File> {
        match self {
            Self::FileSystem(root) => fs::File::open(resolve(root, relative)),
            #[cfg(target_os = "android")]
            Self::AndroidTree(uri) => {
                crate::platform::android::document_tree::access::open_read(uri, relative)
            }
        }
    }

    pub fn write_with_modified_time(
        &self,
        relative: &RelativePath,
        contents: &[u8],
        modified_unix_seconds: Option<i64>,
    ) -> io::Result<()> {
        match self {
            Self::FileSystem(root) => {
                let path = resolve(root, relative);
                atomic_write(&path, contents)?;
                if let Some(seconds) =
                    modified_unix_seconds.and_then(|value| u64::try_from(value).ok())
                    && let Ok(file) = fs::File::options().write(true).open(path)
                {
                    let _ = file.set_times(
                        fs::FileTimes::new()
                            .set_modified(UNIX_EPOCH + Duration::from_secs(seconds)),
                    );
                }
                Ok(())
            }
            #[cfg(target_os = "android")]
            Self::AndroidTree(uri) => {
                let mut file =
                    crate::platform::android::document_tree::access::open_write(uri, relative)?;
                file.write_all(contents)?;
                crate::platform::android::document_tree::file::finish_write(&mut file)
            }
        }
    }

    pub fn copy_from(&self, relative: &RelativePath, source: &Path) -> io::Result<()> {
        match self {
            Self::FileSystem(root) => atomic_write(&resolve(root, relative), &fs::read(source)?),
            #[cfg(target_os = "android")]
            Self::AndroidTree(uri) => {
                let mut source = fs::File::open(source)?;
                let mut destination =
                    crate::platform::android::document_tree::access::open_write(uri, relative)?;
                io::copy(&mut source, &mut destination)?;
                crate::platform::android::document_tree::file::finish_write(&mut destination)
            }
        }
    }

    pub fn create_directory_all(&self, relative: &RelativePath) -> io::Result<()> {
        match self {
            Self::FileSystem(root) => fs::create_dir_all(resolve(root, relative)),
            #[cfg(target_os = "android")]
            Self::AndroidTree(uri) => {
                crate::platform::android::document_tree::access::create_directories(uri, relative)
            }
        }
    }

    pub fn delete(&self, relative: &RelativePath) -> io::Result<()> {
        match self {
            Self::FileSystem(root) => delete_native(&resolve(root, relative)),
            #[cfg(target_os = "android")]
            Self::AndroidTree(uri) => {
                crate::platform::android::document_tree::access::delete(uri, relative)
            }
        }
    }

    pub fn native_path(&self, relative: &RelativePath) -> Option<PathBuf> {
        match self {
            Self::FileSystem(root) => Some(resolve(root, relative)),
            #[cfg(target_os = "android")]
            Self::AndroidTree(_) => None,
        }
    }

    #[cfg(test)]
    pub fn native_root(&self) -> Option<&Path> {
        match self {
            Self::FileSystem(root) => Some(root),
            #[cfg(target_os = "android")]
            Self::AndroidTree(_) => None,
        }
    }
}

fn delete_native(path: &Path) -> io::Result<()> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error),
    };
    if metadata.is_dir() {
        fs::remove_dir(path)
    } else {
        fs::remove_file(path)
    }
}

fn resolve(root: &Path, relative: &RelativePath) -> PathBuf {
    relative
        .as_str()
        .split('/')
        .fold(root.to_owned(), |path, component| path.join(component))
}
