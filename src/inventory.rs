use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RelativePath(String);

impl RelativePath {
    pub fn new(path: impl Into<String>) -> Result<Self, InventoryError> {
        let path = path.into();
        let is_valid = !path.is_empty()
            && !path.starts_with('/')
            && !path.contains('\\')
            && path
                .split('/')
                .all(|component| !component.is_empty() && component != "." && component != "..");

        if is_valid {
            Ok(Self(path))
        } else {
            Err(InventoryError::InvalidRelativePath(path))
        }
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InventoryEntryKind {
    File,
    Directory,
    Symlink { target: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InventoryEntry {
    pub path: RelativePath,
    pub kind: InventoryEntryKind,
    pub byte_size: u64,
    pub modified_unix_seconds: Option<i64>,
}

impl InventoryEntry {
    pub fn new(
        path: RelativePath,
        kind: InventoryEntryKind,
        byte_size: u64,
        modified_unix_seconds: Option<i64>,
    ) -> Self {
        Self {
            path,
            kind,
            byte_size,
            modified_unix_seconds,
        }
    }
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct Inventory {
    entries: BTreeMap<RelativePath, InventoryEntry>,
}

impl Inventory {
    pub fn new(entries: impl IntoIterator<Item = InventoryEntry>) -> Result<Self, InventoryError> {
        let mut inventory = Self::default();
        for entry in entries {
            let path = entry.path.clone();
            if inventory.entries.insert(path.clone(), entry).is_some() {
                return Err(InventoryError::DuplicatePath(path));
            }
        }
        Ok(inventory)
    }

    pub fn entries(&self) -> impl Iterator<Item = &InventoryEntry> {
        self.entries.values()
    }

    /// Reports every pair or group that cannot safely be written to a case-insensitive target.
    pub fn case_collisions(&self) -> Vec<Vec<RelativePath>> {
        let mut groups = BTreeMap::<String, BTreeSet<RelativePath>>::new();
        for path in self.entries.keys() {
            groups
                .entry(path.as_str().to_lowercase())
                .or_default()
                .insert(path.clone());
        }
        groups
            .into_values()
            .filter(|paths| paths.len() > 1)
            .map(|paths| paths.into_iter().collect())
            .collect()
    }

    pub fn validate_case_collisions(&self) -> Result<(), InventoryError> {
        self.case_collisions()
            .into_iter()
            .next()
            .map(InventoryError::CaseCollision)
            .map_or(Ok(()), Err)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum InventoryError {
    InvalidRelativePath(String),
    DuplicatePath(RelativePath),
    CaseCollision(Vec<RelativePath>),
}

impl fmt::Display for InventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRelativePath(path) => write!(formatter, "invalid relative path: {path}"),
            Self::DuplicatePath(path) => {
                write!(formatter, "duplicate inventory path: {}", path.as_str())
            }
            Self::CaseCollision(paths) => write!(
                formatter,
                "paths differ only by case: {}",
                paths
                    .iter()
                    .map(RelativePath::as_str)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl Error for InventoryError {}

#[cfg(test)]
mod tests {
    use super::{Inventory, InventoryEntry, InventoryEntryKind, InventoryError, RelativePath};

    fn entry(path: &str, kind: InventoryEntryKind) -> InventoryEntry {
        InventoryEntry::new(RelativePath::new(path).unwrap(), kind, 0, None)
    }

    #[test]
    fn preserves_case_hidden_paths_empty_directories_and_symlinks() {
        let inventory = Inventory::new([
            entry(".config/Settings", InventoryEntryKind::File),
            entry(".config/settings", InventoryEntryKind::File),
            entry("empty", InventoryEntryKind::Directory),
            entry(
                "current",
                InventoryEntryKind::Symlink {
                    target: "releases/é".into(),
                },
            ),
        ])
        .unwrap();

        assert_eq!(inventory.entries().count(), 4);
        assert_eq!(inventory.case_collisions().len(), 1);
    }

    #[test]
    fn rejects_non_normalized_relative_paths() {
        for path in [
            "",
            "/absolute",
            "folder//file",
            "folder/../file",
            "folder\\file",
        ] {
            assert!(matches!(
                RelativePath::new(path),
                Err(InventoryError::InvalidRelativePath(_))
            ));
        }
    }

    #[test]
    fn rejects_duplicate_paths_and_reports_case_collisions() {
        let duplicate = entry("same", InventoryEntryKind::File);
        assert!(matches!(
            Inventory::new([duplicate.clone(), duplicate]),
            Err(InventoryError::DuplicatePath(_))
        ));

        let inventory = Inventory::new([
            entry("Readme", InventoryEntryKind::File),
            entry("README", InventoryEntryKind::File),
        ])
        .unwrap();
        assert!(matches!(
            inventory.validate_case_collisions(),
            Err(InventoryError::CaseCollision(_))
        ));
    }
}
