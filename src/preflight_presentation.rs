use crate::{
    configuration::SyncMode,
    inventory::InventoryEntryKind,
    planning::{Direction, TransferPlan},
    preflight::Preflight,
    preflight_review::{ReviewItem, ReviewStatus, review_items},
};

/// UI-ready, non-secret information shown after a preflight has completed.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreflightPresentation {
    pub additions: String,
    pub overwrites: String,
    pub deletions: String,
    pub skipped: String,
    pub start_action: &'static str,
    pub requires_mirror_confirmation: bool,
    pub items: Vec<PreflightItemPresentation>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PreflightItemPresentation {
    pub path: String,
    pub status: &'static str,
    pub detail: String,
}

impl From<&Preflight> for PreflightPresentation {
    fn from(preflight: &Preflight) -> Self {
        let summary = preflight.plan().summary();
        Self {
            additions: count_with_size(summary.additions(), "new", "new", summary.copy_byte_size()),
            overwrites: count_with_size(
                summary.overwrites(),
                "overwrite",
                "overwrites",
                summary.overwrite_byte_size(),
            ),
            deletions: count_with_size(
                summary.deletions(),
                "deletion",
                "deletions",
                summary.delete_byte_size(),
            ),
            skipped: count(summary.skipped(), "skipped", "skipped"),
            start_action: start_action(preflight.plan()),
            requires_mirror_confirmation: preflight.plan().mode() == SyncMode::Mirror
                && preflight.plan().requires_confirmation(),
            items: review_items(preflight)
                .into_iter()
                .map(PreflightItemPresentation::from)
                .collect(),
        }
    }
}

impl From<ReviewItem> for PreflightItemPresentation {
    fn from(item: ReviewItem) -> Self {
        Self {
            path: item.path.as_str().into(),
            status: status_label(item.status),
            detail: details(&item),
        }
    }
}

fn details(item: &ReviewItem) -> String {
    [
        item.source.as_ref().map(|entry| describe("Source", entry)),
        item.destination
            .as_ref()
            .map(|entry| describe("Destination", entry)),
    ]
    .into_iter()
    .flatten()
    .collect::<Vec<_>>()
    .join(" · ")
}

fn describe(label: &str, entry: &crate::preflight_review::ReviewEntryDetails) -> String {
    format!(
        "{label}: {} · {}",
        kind_label(&entry.kind),
        bytes(entry.byte_size)
    )
}

fn kind_label(kind: &InventoryEntryKind) -> &'static str {
    match kind {
        InventoryEntryKind::File => "file",
        InventoryEntryKind::Directory => "folder",
        InventoryEntryKind::Symlink { .. } => "link",
    }
}

fn bytes(size: u64) -> String {
    if size == 1 {
        "1 byte".into()
    } else {
        format!("{size} bytes")
    }
}

fn count(count: usize, singular: &str, plural: &str) -> String {
    let label = if count == 1 { singular } else { plural };
    format!("{count} {label}")
}

fn count_with_size(item_count: usize, singular: &str, plural: &str, byte_size: u64) -> String {
    format!(
        "{} · {}",
        count(item_count, singular, plural),
        bytes(byte_size)
    )
}

fn start_action(plan: &TransferPlan) -> &'static str {
    match plan.mode() {
        SyncMode::Archive => "Create archive",
        SyncMode::Mirror => "Start mirror",
        SyncMode::AddOnly => match plan.direction() {
            Direction::Upload => "Start upload",
            Direction::Download => "Start download",
            Direction::BothWays => "Start both ways",
        },
    }
}

fn status_label(status: ReviewStatus) -> &'static str {
    match status {
        ReviewStatus::New => "New",
        ReviewStatus::Unchanged => "Unchanged",
        ReviewStatus::Changed => "Changed",
        ReviewStatus::WillOverwrite => "Will overwrite",
        ReviewStatus::WillDelete => "Will delete",
        ReviewStatus::Warning => "Warning",
    }
}

#[cfg(test)]
#[path = "preflight_presentation_tests.rs"]
mod tests;
