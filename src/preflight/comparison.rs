use crate::inventory::{Inventory, InventoryEntry, InventoryEntryKind, RelativePath};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EntryStatus {
    New,
    Unchanged,
    Changed,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ComparedEntry {
    pub path: RelativePath,
    pub status: EntryStatus,
    pub source: Option<InventoryEntry>,
    pub destination: Option<InventoryEntry>,
}

/// Compares two read-only inventories without opening any file contents.
pub fn compare(source: &Inventory, destination: &Inventory) -> Vec<ComparedEntry> {
    let mut entries = Vec::new();
    for source_entry in source.entries() {
        let destination_entry = destination.get(&source_entry.path);
        entries.push(ComparedEntry {
            path: source_entry.path.clone(),
            status: destination_entry.map_or(EntryStatus::New, |entry| {
                status_for_matching_paths(source_entry, entry)
            }),
            source: Some(source_entry.clone()),
            destination: destination_entry.cloned(),
        });
    }
    for destination_entry in destination.entries() {
        if source.get(&destination_entry.path).is_none() {
            entries.push(ComparedEntry {
                path: destination_entry.path.clone(),
                status: EntryStatus::New,
                source: None,
                destination: Some(destination_entry.clone()),
            });
        }
    }
    entries.sort_by(|left, right| left.path.cmp(&right.path));
    entries
}

fn status_for_matching_paths(source: &InventoryEntry, destination: &InventoryEntry) -> EntryStatus {
    let unchanged = match (&source.kind, &destination.kind) {
        (InventoryEntryKind::Directory, InventoryEntryKind::Directory) => true,
        (
            InventoryEntryKind::Symlink { target: left },
            InventoryEntryKind::Symlink { target: right },
        ) => left == right,
        (InventoryEntryKind::File, InventoryEntryKind::File) => {
            return file_status(source, destination);
        }
        _ => false,
    };
    if unchanged {
        EntryStatus::Unchanged
    } else {
        EntryStatus::Changed
    }
}

fn file_status(source: &InventoryEntry, destination: &InventoryEntry) -> EntryStatus {
    if source.byte_size != destination.byte_size {
        return EntryStatus::Changed;
    }
    EntryStatus::Unchanged
}

#[cfg(test)]
mod tests;
