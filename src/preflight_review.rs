use crate::comparison::{ComparedEntry, EntryStatus};
use crate::inventory::{InventoryEntry, InventoryEntryKind, RelativePath};
use crate::planning::PlannedAction;
use crate::preflight::Preflight;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReviewStatus {
    New,
    Unchanged,
    Changed,
    WillOverwrite,
    WillDelete,
    Warning,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewItem {
    pub path: RelativePath,
    pub status: ReviewStatus,
    pub source: Option<ReviewEntryDetails>,
    pub destination: Option<ReviewEntryDetails>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ReviewEntryDetails {
    pub kind: InventoryEntryKind,
    pub byte_size: u64,
    pub modified_unix_seconds: Option<i64>,
}

impl From<&InventoryEntry> for ReviewEntryDetails {
    fn from(entry: &InventoryEntry) -> Self {
        Self {
            kind: entry.kind.clone(),
            byte_size: entry.byte_size,
            modified_unix_seconds: entry.modified_unix_seconds,
        }
    }
}

pub fn review_items(preflight: &Preflight) -> Vec<ReviewItem> {
    preflight
        .comparison()
        .iter()
        .filter_map(|entry| {
            status_for(entry, preflight.plan().actions()).map(|status| ReviewItem {
                path: entry.path.clone(),
                status,
                source: entry.source.as_ref().map(ReviewEntryDetails::from),
                destination: entry.destination.as_ref().map(ReviewEntryDetails::from),
            })
        })
        .collect()
}

fn status_for(entry: &ComparedEntry, actions: &[PlannedAction]) -> Option<ReviewStatus> {
    let action = actions.iter().find(|action| match action {
        PlannedAction::Copy { path, .. }
        | PlannedAction::Overwrite { path, .. }
        | PlannedAction::Delete { path, .. }
        | PlannedAction::SkipChanged { path } => path == &entry.path,
        PlannedAction::CreateArchive { .. } => false,
    });
    match action {
        Some(PlannedAction::Copy { .. }) => Some(ReviewStatus::New),
        Some(PlannedAction::Overwrite { .. }) => Some(ReviewStatus::WillOverwrite),
        Some(PlannedAction::Delete { .. }) => Some(ReviewStatus::WillDelete),
        Some(PlannedAction::SkipChanged { .. }) => Some(ReviewStatus::Warning),
        Some(PlannedAction::CreateArchive { .. }) => None,
        None => match entry.status {
            EntryStatus::Unchanged => Some(ReviewStatus::Unchanged),
            EntryStatus::Changed => Some(ReviewStatus::Changed),
            EntryStatus::New if entry.source.is_some() => Some(ReviewStatus::New),
            EntryStatus::New => None,
        },
    }
}

#[cfg(test)]
#[path = "preflight_review_tests.rs"]
mod tests;
