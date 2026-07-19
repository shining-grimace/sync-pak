use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
};

use crate::inventory::RelativePath;

/// Resolves inventory paths beneath a configured local root without lexical traversal.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalTransferRoot(PathBuf);

impl LocalTransferRoot {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self(root.into())
    }

    pub fn resolve(&self, relative: &RelativePath) -> PathBuf {
        relative
            .as_str()
            .split('/')
            .fold(self.0.clone(), |path, component| path.join(component))
    }

    pub fn as_path(&self) -> &Path {
        &self.0
    }
}

/// A normalized provider key prefix for a configured connection.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteTransferPrefix(String);

impl RemoteTransferPrefix {
    pub fn new(prefix: &str) -> Result<Self, TransferPathError> {
        if prefix.is_empty() {
            return Ok(Self(String::new()));
        }
        let prefix = prefix.trim_end_matches('/');
        if prefix.is_empty() {
            return Err(TransferPathError::InvalidRemotePrefix(prefix.into()));
        }
        RelativePath::new(prefix)
            .map_err(|_| TransferPathError::InvalidRemotePrefix(prefix.into()))?;
        Ok(Self(format!("{prefix}/")))
    }

    pub fn resolve(&self, relative: &RelativePath) -> String {
        format!("{}{}", self.0, relative.as_str())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TransferPathError {
    InvalidRemotePrefix(String),
}

impl fmt::Display for TransferPathError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRemotePrefix(prefix) => {
                write!(formatter, "invalid remote prefix: {prefix}")
            }
        }
    }
}

impl Error for TransferPathError {}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use crate::inventory::RelativePath;

    use super::{LocalTransferRoot, RemoteTransferPrefix, TransferPathError};

    #[test]
    fn resolves_relative_paths_beneath_local_and_remote_roots() {
        let path = RelativePath::new("photos/é.png").unwrap();

        assert_eq!(
            LocalTransferRoot::new("/sync").resolve(&path),
            Path::new("/sync/photos/é.png")
        );
        assert_eq!(
            RemoteTransferPrefix::new("backups/")
                .unwrap()
                .resolve(&path),
            "backups/photos/é.png"
        );
    }

    #[test]
    fn rejects_non_normalized_remote_prefixes() {
        assert_eq!(
            RemoteTransferPrefix::new("../outside"),
            Err(TransferPathError::InvalidRemotePrefix("../outside".into()))
        );
        assert_eq!(RemoteTransferPrefix::new("").unwrap().as_str(), "");
    }
}
