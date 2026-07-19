use crate::{execution::ExecutionState, planning::PlannedAction};

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

pub(crate) struct NoopProgressObserver;

impl TransferProgressObserver for NoopProgressObserver {
    fn on_progress(&self, _: &TransferProgress) {}
}
