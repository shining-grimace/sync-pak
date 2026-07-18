use std::path::Path;

use crate::capabilities::CapabilityError;

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

pub fn utf8_path(path: &Path) -> Result<&str, CapabilityError> {
    path.to_str().ok_or(CapabilityError::UnsupportedPath)
}

#[cfg(test)]
mod tests {
    use super::utf8_path;
    use std::path::Path;

    #[test]
    fn utf8_paths_are_preserved() {
        assert_eq!(utf8_path(Path::new("/files/é")), Ok("/files/é"));
    }
}
