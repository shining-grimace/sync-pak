use std::path::PathBuf;

pub trait ProtectedCredentialStore {
    fn save(&self, provider_id: &str, credential_json: &[u8]) -> Result<(), CapabilityError>;
    fn load(&self, provider_id: &str) -> Result<Vec<u8>, CapabilityError>;
    fn delete(&self, provider_id: &str) -> Result<(), CapabilityError>;
}

pub trait FolderPicker {
    fn pick_folder(&self, completion: FolderPickerCompletion) -> Result<(), CapabilityError>;
}

pub type FolderPickerCompletion =
    Box<dyn FnOnce(Result<Option<FolderSelection>, CapabilityError>) + Send + 'static>;

pub trait DesktopNotifier {
    fn show(&self, notification: &DesktopNotification<'_>) -> Result<(), CapabilityError>;
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DesktopNotification<'a> {
    pub title: &'a str,
    pub body: &'a str,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum FolderSelection {
    FileSystem(PathBuf),
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
    Busy,
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
            Self::Busy => "Another operating system request is already in progress.",
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

#[cfg(test)]
mod tests {
    use super::FolderSelection;

    #[test]
    fn android_tree_uri_is_preserved_exactly() {
        let uri = "content://com.android.externalstorage.documents/tree/primary%3ADocuments";
        let selection = FolderSelection::AndroidTreeUri(uri.to_owned());

        assert_eq!(selection.display_value(), Ok(uri));
    }
}
