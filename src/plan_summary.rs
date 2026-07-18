use crate::comparison::ComparedEntry;
use crate::planning::{Endpoint, PlannedAction};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct PlanSummary {
    additions: usize,
    overwrites: usize,
    deletions: usize,
    skipped: usize,
    archives: usize,
    copy_byte_size: u64,
    overwrite_byte_size: u64,
    delete_byte_size: u64,
}

impl PlanSummary {
    pub fn additions(&self) -> usize {
        self.additions
    }

    pub fn overwrites(&self) -> usize {
        self.overwrites
    }

    pub fn deletions(&self) -> usize {
        self.deletions
    }

    pub fn skipped(&self) -> usize {
        self.skipped
    }

    pub fn archives(&self) -> usize {
        self.archives
    }

    pub fn copy_byte_size(&self) -> u64 {
        self.copy_byte_size
    }

    pub fn overwrite_byte_size(&self) -> u64 {
        self.overwrite_byte_size
    }

    pub fn delete_byte_size(&self) -> u64 {
        self.delete_byte_size
    }
}

pub(crate) fn summarize(actions: &[PlannedAction], comparison: &[ComparedEntry]) -> PlanSummary {
    let mut summary = PlanSummary::default();
    for action in actions {
        match action {
            PlannedAction::Copy { path, from, .. } => {
                summary.additions += 1;
                summary.copy_byte_size += byte_size(path, *from, comparison);
            }
            PlannedAction::Overwrite { path, from, .. } => {
                summary.overwrites += 1;
                summary.overwrite_byte_size += byte_size(path, *from, comparison);
            }
            PlannedAction::Delete { path, from } => {
                summary.deletions += 1;
                summary.delete_byte_size += byte_size(path, *from, comparison);
            }
            PlannedAction::SkipChanged { .. } => summary.skipped += 1,
            PlannedAction::CreateArchive { .. } => summary.archives += 1,
        }
    }
    summary
}

fn byte_size(
    path: &crate::inventory::RelativePath,
    endpoint: Endpoint,
    comparison: &[ComparedEntry],
) -> u64 {
    comparison
        .iter()
        .find(|entry| entry.path == *path)
        .and_then(|entry| match endpoint {
            Endpoint::Source => entry.source.as_ref(),
            Endpoint::Destination => entry.destination.as_ref(),
        })
        .map(|entry| entry.byte_size)
        .unwrap_or(0)
}
