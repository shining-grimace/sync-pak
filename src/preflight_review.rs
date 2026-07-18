use crate::comparison::{ComparedEntry, EntryStatus};
use crate::inventory::RelativePath;
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
}

pub fn review_items(preflight: &Preflight) -> Vec<ReviewItem> {
    preflight
        .comparison()
        .iter()
        .filter_map(|entry| {
            status_for(entry, preflight.plan().actions()).map(|status| ReviewItem {
                path: entry.path.clone(),
                status,
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
