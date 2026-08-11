use std::path::PathBuf;

use super::CapabilityError;

pub trait FolderPicker {
    fn pick_folder(&self, completion: FolderPickerCompletion) -> Result<(), CapabilityError>;
}

pub type FolderPickerCompletion =
    Box<dyn FnOnce(Result<Option<FolderSelection>, CapabilityError>) + Send + 'static>;

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
