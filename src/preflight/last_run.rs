//! Connection-wide comparison cutoff; no content reads or per-file history.
use super::{Preflight, comparison::EntryStatus, planning::plan};
use crate::inventory::InventoryEntryKind;

impl Preflight {
    pub(crate) fn with_last_run(mut self, last_run: Option<u64>) -> Self {
        for entry in &mut self.comparison {
            let (Some(source), Some(destination)) = (&entry.source, &entry.destination) else {
                continue;
            };
            if source.kind != InventoryEntryKind::File
                || destination.kind != InventoryEntryKind::File
            {
                continue;
            }
            let accepted = match (
                last_run,
                source.modified_unix_seconds,
                destination.modified_unix_seconds,
            ) {
                (Some(cutoff), Some(left), Some(right)) => {
                    i128::from(left) <= i128::from(cutoff)
                        && i128::from(right) <= i128::from(cutoff)
                }
                _ => false,
            };
            let same_time = matches!(
                (source.modified_unix_seconds, destination.modified_unix_seconds),
                (Some(left), Some(right)) if left == right
            );
            entry.status = if source.byte_size == destination.byte_size && (same_time || accepted) {
                EntryStatus::Unchanged
            } else {
                EntryStatus::Changed
            };
        }
        self.plan = plan(self.plan.mode(), self.plan.direction(), &self.comparison)
            .expect("the preflight direction has already been validated");
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        configuration::SyncMode,
        inventory::{Inventory, InventoryEntry, RelativePath},
        preflight::planning::Direction,
        preflight::{CaseSensitivity, preflight},
    };
    #[test]
    fn cutoff_requires_both_known_timestamps_and_equal_sizes() {
        for (cutoff, left, right, size, expected) in [
            (None, Some(1), Some(1), 1, EntryStatus::Unchanged),
            (Some(10), Some(11), Some(11), 1, EntryStatus::Unchanged),
            (None, Some(1), Some(2), 1, EntryStatus::Changed),
            (None, None, None, 1, EntryStatus::Changed),
            (Some(10), Some(11), Some(11), 2, EntryStatus::Changed),
            (Some(10), None, Some(1), 1, EntryStatus::Changed),
            (Some(10), Some(1), None, 1, EntryStatus::Changed),
            (Some(10), Some(11), Some(1), 1, EntryStatus::Changed),
            (Some(10), Some(1), Some(11), 1, EntryStatus::Changed),
            (Some(10), Some(10), Some(1), 1, EntryStatus::Unchanged),
            (Some(10), Some(1), Some(1), 2, EntryStatus::Changed),
        ] {
            let inventory = |size, time| {
                Inventory::new([InventoryEntry::new(
                    RelativePath::new("file").unwrap(),
                    InventoryEntryKind::File,
                    size,
                    time,
                )])
                .unwrap()
            };
            let result = preflight(
                SyncMode::Mirror,
                Direction::Upload,
                &inventory(1, left),
                CaseSensitivity::Sensitive,
                &inventory(size, right),
                CaseSensitivity::Sensitive,
            )
            .unwrap()
            .with_last_run(cutoff);
            assert_eq!(result.comparison()[0].status, expected);
            assert_eq!(
                result.plan().summary().overwrites(),
                usize::from(expected == EntryStatus::Changed)
            );
        }
    }
}
