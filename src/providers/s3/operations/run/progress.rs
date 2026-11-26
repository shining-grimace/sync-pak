use crate::{
    configuration::SyncMode,
    inventory::InventoryEntryKind,
    operations::transfer::{
        modes::mirror::ordered_actions,
        progress::{
            TransferProgress, TransferProgressObserver, TransferWork, TransferWorkProgress,
        },
        upload_strategy::{UploadStrategy, select_upload_strategy},
    },
    preflight::{
        Preflight,
        planning::{Direction, Endpoint, PlannedAction},
    },
};

pub(super) struct WeightedProgress<'a> {
    observer: &'a dyn TransferProgressObserver,
    cumulative_work: Vec<TransferWork>,
}

impl<'a> WeightedProgress<'a> {
    pub(super) fn new(
        preflight: &Preflight,
        cache_enabled: bool,
        observer: &'a dyn TransferProgressObserver,
    ) -> Self {
        let plan = preflight.plan();
        let actions = match plan.mode() {
            SyncMode::Mirror => ordered_actions(plan.actions()),
            _ => plan.actions().iter().collect(),
        };
        let mut cumulative_work = Vec::with_capacity(actions.len() + 1);
        let mut total = TransferWork::default();
        cumulative_work.push(total);
        for action in actions {
            let work = action_work(preflight, action, cache_enabled);
            total.bytes = total.bytes.saturating_add(work.bytes);
            total.network_requests = total.network_requests.saturating_add(work.network_requests);
            cumulative_work.push(total);
        }
        Self {
            observer,
            cumulative_work,
        }
    }
}

impl TransferProgressObserver for WeightedProgress<'_> {
    fn on_progress(&self, progress: &TransferProgress) {
        let mut progress = progress.clone();
        progress.work = Some(TransferWorkProgress {
            completed: self.cumulative_work[progress.completed_actions],
            total: self.cumulative_work[progress.total_actions],
        });
        self.observer.on_progress(&progress);
    }
}

fn action_work(preflight: &Preflight, action: &PlannedAction, cache_enabled: bool) -> TransferWork {
    let remote = match preflight.plan().direction() {
        Direction::Download => Endpoint::Source,
        Direction::Upload | Direction::BothWays => Endpoint::Destination,
    };
    let (bytes, network_requests) = match action {
        PlannedAction::Copy { path, from, .. } | PlannedAction::Overwrite { path, from, .. } => {
            let comparison = preflight.comparison();
            let entry = comparison
                .binary_search_by(|entry| entry.path.cmp(path))
                .ok()
                .and_then(|index| match from {
                    Endpoint::Source => comparison[index].source.as_ref(),
                    Endpoint::Destination => comparison[index].destination.as_ref(),
                });
            let file = entry.filter(|entry| entry.kind == InventoryEntryKind::File);
            let bytes = file.map_or(0, |entry| entry.byte_size);
            let requests = if *from == remote {
                // A remote directory can require a prefix lookup after the object lookup.
                if file.is_some() { 1 } else { 2 }
            } else {
                let upload = match select_upload_strategy(bytes) {
                    UploadStrategy::SinglePart => 1,
                    UploadStrategy::Multipart { part_size } => bytes.div_ceil(part_size as u64) + 2,
                };
                upload + u64::from(cache_enabled && file.is_some())
            };
            (bytes, requests)
        }
        PlannedAction::Delete { from, .. } => (0, u64::from(*from == remote)),
        // Accepting an existing pair can fetch metadata when the cache lacks it.
        PlannedAction::SkipChanged { .. } => (0, u64::from(cache_enabled)),
        PlannedAction::CreateArchive { .. } => (0, 0),
    };
    TransferWork {
        bytes,
        network_requests,
    }
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use crate::{
        inventory::{Inventory, InventoryEntry, RelativePath},
        operations::{execution::ExecutionState, queue::progress_observer::from_transfer},
        preflight::{CaseSensitivity, preflight},
    };

    use super::*;

    impl<F: Fn(&TransferProgress)> TransferProgressObserver for F {
        fn on_progress(&self, progress: &TransferProgress) {
            self(progress);
        }
    }

    #[test]
    fn mirror_progress_weights_bytes_and_remote_deletes_in_execution_order() {
        let file = |path, bytes| {
            InventoryEntry::new(
                RelativePath::new(path).unwrap(),
                InventoryEntryKind::File,
                bytes,
                None,
            )
        };
        let local = Inventory::new([file("b-small", 0), file("c-large", 1024 * 1024)]).unwrap();
        let remote = Inventory::new([file("a-delete", 8 * 1024 * 1024)]).unwrap();
        let preflight = preflight(
            SyncMode::Mirror,
            Direction::Upload,
            &local,
            CaseSensitivity::Sensitive,
            &remote,
            CaseSensitivity::Sensitive,
        )
        .unwrap();

        let observed = RefCell::new(Vec::new());
        let observer = |event: &TransferProgress| {
            observed.borrow_mut().push(from_transfer(event));
        };
        let progress = WeightedProgress::new(&preflight, false, &observer);
        for completed_actions in 0..=3 {
            progress.on_progress(&TransferProgress {
                state: ExecutionState::Copying,
                completed_actions,
                total_actions: 3,
                current_action: None,
                work: None,
            });
        }

        let observed = observed.into_inner();
        assert_eq!(
            observed.iter().map(|p| p.percent()).collect::<Vec<_>>(),
            [0, 14, 85, 100]
        );
        assert_eq!(observed[1].transferred_bytes, 0);
        assert_eq!(observed[2].transferred_bytes, 1024 * 1024);
        assert_eq!(observed[2].total_bytes, 1024 * 1024);
    }
}
