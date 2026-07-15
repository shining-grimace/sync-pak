pub trait ProtectedCredentialStore {
    fn save(&self, provider_id: &str, credential_json: &[u8]) -> Result<(), CapabilityError>;
    fn load(&self, provider_id: &str) -> Result<Vec<u8>, CapabilityError>;
    fn delete(&self, provider_id: &str) -> Result<(), CapabilityError>;
}

pub trait FolderPicker {
    fn pick_folder(&self) -> Result<Option<FolderSelection>, CapabilityError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderSelection {
    FileSystem(PathBuf),
    #[allow(dead_code, reason = "constructed by the planned Android SAF adapter")]
    AndroidTreeUri(String),
}

impl FolderSelection {
    pub fn display_value(&self) -> Result<&str, CapabilityError> {
        match self {
            Self::FileSystem(path) => path.to_str().ok_or(CapabilityError::UnsupportedPath),
            Self::AndroidTreeUri(uri) => Ok(uri),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CapabilityError {
    InvalidReference,
    NotFound,
    Unsupported,
    UnsupportedPath,
    Unavailable,
    Unexpected,
}

impl std::fmt::Display for CapabilityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidReference => "The protected credential reference is invalid.",
            Self::NotFound => "The protected credential was not found.",
            Self::Unsupported => "This capability is not implemented on this platform yet.",
            Self::UnsupportedPath => "The selected folder cannot be represented safely as UTF-8.",
            Self::Unavailable => "The operating system facility is locked or unavailable.",
            Self::Unexpected => "The operating system could not complete the request.",
        })
    }
}

impl std::error::Error for CapabilityError {}
use std::path::PathBuf;
