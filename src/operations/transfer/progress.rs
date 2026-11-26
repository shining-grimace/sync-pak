use crate::{operations::execution::ExecutionState, preflight::planning::PlannedAction};

/// A UI-safe snapshot of serial transfer execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferProgress {
    pub state: ExecutionState,
    pub completed_actions: usize,
    pub total_actions: usize,
    pub current_action: Option<PlannedAction>,
    pub work: Option<TransferWorkProgress>,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct TransferWork {
    pub bytes: u64,
    pub network_requests: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TransferWorkProgress {
    pub completed: TransferWork,
    pub total: TransferWork,
}

impl TransferWorkProgress {
    pub fn percent(self) -> Option<i32> {
        // Give a request the same weight as transferring 256 KiB.
        const BYTES_PER_REQUEST: u128 = 256 * 1024;
        let weight = |work: TransferWork| {
            u128::from(work.bytes) + u128::from(work.network_requests) * BYTES_PER_REQUEST
        };
        let total = weight(self.total);
        (weight(self.completed).min(total) * 100)
            .checked_div(total)
            .map(|percent| percent as i32)
    }
}

pub trait TransferProgressObserver {
    fn on_progress(&self, progress: &TransferProgress);
}

#[cfg(test)]
pub struct NoopProgressObserver;

#[cfg(test)]
impl TransferProgressObserver for NoopProgressObserver {
    fn on_progress(&self, _: &TransferProgress) {}
}
