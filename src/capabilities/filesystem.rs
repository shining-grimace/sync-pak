use std::path::Path;

use super::CapabilityError;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FileMetadata {
    pub byte_size: u64,
    pub modified_unix_seconds: Option<i64>,
    pub is_directory: bool,
}

pub trait FileSystemAccess {
    fn metadata(&self, path: &Path) -> Result<FileMetadata, CapabilityError>;
    fn create_directory_all(&self, path: &Path) -> Result<(), CapabilityError>;
}
