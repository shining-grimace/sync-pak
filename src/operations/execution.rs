mod summary;
use std::collections::VecDeque;

use crate::{
    capabilities::CapabilityError, operations::queue::QueueEntry,
    operations::transfer::progress::TransferProgressObserver, preflight::planning::PlannedAction,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum ExecutionState {
    #[allow(
        dead_code,
        reason = "reserved for executors that report preparation progress"
    )]
    Preparing,
    Copying,
    #[allow(
        dead_code,
        reason = "reserved for executors that report finalization progress"
    )]
    Finalizing,
    Completed,
    Failed,
    Cancelled,
}

/// The immutable outcome of an operation, including work left by a partial run.
#[derive(Clone, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub struct ExecutionResult {
    pub state: ExecutionState,
    pub failure_message: Option<String>,
    pub completed: Vec<PlannedAction>,
    pub incomplete: Vec<PlannedAction>,
    pub not_started: Vec<PlannedAction>,
}

impl ExecutionResult {
    /// Creates the result for an operation cancelled before any action began.
    pub fn cancelled_before_start() -> Self {
        Self::before_start(ExecutionState::Cancelled)
    }

    /// Creates the result for an operation that failed before any action began.
    pub fn failed_before_start() -> Self {
        Self::before_start(ExecutionState::Failed)
    }

    /// Creates a failed result while retaining a safe, user-facing reason.
    pub fn failed_before_start_with_message(message: impl Into<String>) -> Self {
        Self {
            failure_message: Some(message.into()),
            ..Self::failed_before_start()
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            ExecutionState::Completed | ExecutionState::Failed | ExecutionState::Cancelled
        )
    }

    fn before_start(state: ExecutionState) -> Self {
        Self {
            state,
            failure_message: None,
            completed: Vec::new(),
            incomplete: Vec::new(),
            not_started: Vec::new(),
        }
    }
}

/// Tracks a serial operation's planned actions while it is running.
pub struct ExecutionProgress {
    completed: Vec<PlannedAction>,
    current: Option<PlannedAction>,
    pending: VecDeque<PlannedAction>,
}

impl ExecutionProgress {
    pub fn new(actions: impl IntoIterator<Item = PlannedAction>) -> Self {
        Self {
            completed: Vec::new(),
            current: None,
            pending: actions.into_iter().collect(),
        }
    }

    /// Starts the next action, if there is one and no other action is in progress.
    pub fn start_next(&mut self) -> Option<&PlannedAction> {
        if self.current.is_none() {
            self.current = self.pending.pop_front();
        }
        self.current.as_ref()
    }

    /// Records the current action as complete.
    pub fn complete_current(&mut self) -> bool {
        match self.current.take() {
            Some(action) => {
                self.completed.push(action);
                true
            }
            None => false,
        }
    }

    pub fn finish(self) -> ExecutionResult {
        self.into_result(ExecutionState::Completed)
    }

    pub fn cancel(self) -> ExecutionResult {
        self.into_result(ExecutionState::Cancelled)
    }

    pub fn fail(self) -> ExecutionResult {
        self.into_result(ExecutionState::Failed)
    }

    fn into_result(self, state: ExecutionState) -> ExecutionResult {
        ExecutionResult {
            state,
            failure_message: None,
            completed: self.completed,
            incomplete: self.current.into_iter().collect(),
            not_started: self.pending.into_iter().collect(),
        }
    }
}

pub trait OperationExecutor {
    /// Runs one queued operation and reports its non-secret progress while it is active.
    fn execute(
        &self,
        entry: &QueueEntry,
        observer: &dyn TransferProgressObserver,
    ) -> Result<ExecutionResult, CapabilityError>;
    fn cancel(&self, connection_id: &str) -> Result<(), CapabilityError>;
}

#[cfg(test)]
mod tests;
