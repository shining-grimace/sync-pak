use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;

use crate::inventory::{
    Inventory, InventoryEntry, InventoryEntryKind, InventoryError, RelativePath,
};
use crate::provider_capabilities::{ObjectLister, ProviderError, RemoteObject};

pub async fn list_remote_inventory<L: ObjectLister + ?Sized>(
    lister: &L,
    bucket: &str,
    prefix: &str,
) -> Result<Inventory, RemoteInventoryError> {
    let prefix = normalize_prefix(prefix)?;
    let objects = lister
        .list_objects(bucket, &prefix)
        .await
        .map_err(RemoteInventoryError::Provider)?;
    inventory_from_objects(&prefix, objects)
}

pub fn inventory_from_objects(
    prefix: &str,
    objects: impl IntoIterator<Item = RemoteObject>,
) -> Result<Inventory, RemoteInventoryError> {
    let prefix = normalize_prefix(prefix)?;
    let mut entries = BTreeMap::new();
    for object in objects {
        let relative = object
            .key
            .strip_prefix(&prefix)
            .ok_or_else(|| RemoteInventoryError::ObjectOutsidePrefix(object.key.clone()))?;
        if relative.is_empty() && object.metadata.byte_size == 0 {
            continue;
        }
        let (relative, kind) = if let Some(path) = relative.strip_suffix('/') {
            if object.metadata.byte_size != 0 || path.is_empty() {
                return Err(RemoteInventoryError::InvalidObjectKey(object.key));
            }
            (
                RelativePath::new(path).map_err(RemoteInventoryError::Inventory)?,
                InventoryEntryKind::Directory,
            )
        } else {
            (
                RelativePath::new(relative).map_err(RemoteInventoryError::Inventory)?,
                InventoryEntryKind::File,
            )
        };
        insert_entry(
            &mut entries,
            InventoryEntry::new(
                relative.clone(),
                kind,
                object.metadata.byte_size,
                object.metadata.modified_unix_seconds,
            ),
        )?;
        insert_parent_directories(&mut entries, &relative)?;
    }
    Inventory::new(entries.into_values()).map_err(RemoteInventoryError::Inventory)
}

fn normalize_prefix(prefix: &str) -> Result<String, RemoteInventoryError> {
    if prefix.is_empty() {
        return Ok(String::new());
    }
    let path = prefix.trim_end_matches('/');
    if path.is_empty() {
        return Err(RemoteInventoryError::InvalidPrefix(prefix.to_owned()));
    }
    RelativePath::new(path).map_err(|_| RemoteInventoryError::InvalidPrefix(prefix.to_owned()))?;
    Ok(format!("{path}/"))
}

fn insert_parent_directories(
    entries: &mut BTreeMap<RelativePath, InventoryEntry>,
    path: &RelativePath,
) -> Result<(), RemoteInventoryError> {
    let components = path.as_str().split('/').collect::<Vec<_>>();
    for index in 1..components.len() {
        let parent = RelativePath::new(components[..index].join("/"))
            .map_err(RemoteInventoryError::Inventory)?;
        insert_entry(
            entries,
            InventoryEntry::new(parent, InventoryEntryKind::Directory, 0, None),
        )?;
    }
    Ok(())
}

fn insert_entry(
    entries: &mut BTreeMap<RelativePath, InventoryEntry>,
    entry: InventoryEntry,
) -> Result<(), RemoteInventoryError> {
    match entries.get(&entry.path) {
        None => {
            entries.insert(entry.path.clone(), entry);
            Ok(())
        }
        Some(existing) if existing == &entry => Ok(()),
        Some(existing)
            if matches!(existing.kind, InventoryEntryKind::Directory)
                && matches!(entry.kind, InventoryEntryKind::Directory) =>
        {
            Ok(())
        }
        Some(_) => Err(RemoteInventoryError::ConflictingEntry(entry.path)),
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum RemoteInventoryError {
    InvalidPrefix(String),
    InvalidObjectKey(String),
    ObjectOutsidePrefix(String),
    ConflictingEntry(RelativePath),
    Inventory(InventoryError),
    Provider(ProviderError),
}

impl fmt::Display for RemoteInventoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidPrefix(prefix) => write!(formatter, "invalid remote prefix: {prefix}"),
            Self::InvalidObjectKey(key) => write!(formatter, "invalid remote object key: {key}"),
            Self::ObjectOutsidePrefix(key) => {
                write!(
                    formatter,
                    "remote object is outside the configured prefix: {key}"
                )
            }
            Self::ConflictingEntry(path) => write!(
                formatter,
                "remote objects conflict at the same relative path: {}",
                path.as_str()
            ),
            Self::Inventory(error) => error.fmt(formatter),
            Self::Provider(error) => error.fmt(formatter),
        }
    }
}

impl Error for RemoteInventoryError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            Self::Inventory(error) => Some(error),
            Self::Provider(error) => Some(error),
            _ => None,
        }
    }
}

#[cfg(test)]
#[path = "remote_inventory_tests.rs"]
mod tests;
