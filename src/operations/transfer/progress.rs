use crate::{operations::execution::ExecutionState, preflight::planning::PlannedAction};

/// A UI-safe snapshot of serial transfer execution.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferProgress {
    pub state: ExecutionState,
    pub completed_actions: usize,
    pub total_actions: usize,
    pub current_action: Option<PlannedAction>,
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
