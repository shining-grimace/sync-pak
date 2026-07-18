use std::collections::VecDeque;

use crate::{
    capabilities::CapabilityError,
    planning::{OperationPlan, PlannedAction},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ExecutionState {
    Preparing,
    Copying,
    Finalizing,
    Completed,
    Failed,
    Cancelled,
}

/// The immutable outcome of an operation, including work left by a partial run.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExecutionResult {
    pub state: ExecutionState,
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

    pub fn is_terminal(&self) -> bool {
        matches!(
            self.state,
            ExecutionState::Completed | ExecutionState::Failed | ExecutionState::Cancelled
        )
    }

    fn before_start(state: ExecutionState) -> Self {
        Self {
            state,
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
            completed: self.completed,
            incomplete: self.current.into_iter().collect(),
            not_started: self.pending.into_iter().collect(),
        }
    }
}

pub trait OperationExecutor {
    fn execute(&self, plan: &OperationPlan) -> Result<ExecutionResult, CapabilityError>;
    fn cancel(&self, connection_id: &str) -> Result<(), CapabilityError>;
}

#[cfg(test)]
mod tests {
    use crate::{
        inventory::RelativePath,
        planning::{Endpoint, PlannedAction},
    };

    use super::{ExecutionProgress, ExecutionState};

    fn actions() -> Vec<PlannedAction> {
        ["first", "second", "third"]
            .into_iter()
            .map(|path| PlannedAction::Copy {
                path: RelativePath::new(path).unwrap(),
                from: Endpoint::Source,
                to: Endpoint::Destination,
            })
            .collect()
    }

    #[test]
    fn cancellation_keeps_completed_incomplete_and_not_started_actions_separate() {
        let mut progress = ExecutionProgress::new(actions());
        progress.start_next();
        assert!(progress.complete_current());
        progress.start_next();

        let result = progress.cancel();

        assert_eq!(result.state, ExecutionState::Cancelled);
        assert_eq!(result.completed.len(), 1);
        assert_eq!(result.incomplete.len(), 1);
        assert_eq!(result.not_started.len(), 1);
    }

    #[test]
    fn completed_run_has_no_remaining_actions() {
        let mut progress = ExecutionProgress::new(actions());
        while progress.start_next().is_some() {
            assert!(progress.complete_current());
        }

        let result = progress.finish();

        assert_eq!(result.state, ExecutionState::Completed);
        assert!(result.incomplete.is_empty());
        assert!(result.not_started.is_empty());
        assert_eq!(result.completed.len(), 3);
    }

    #[test]
    fn cancellation_before_start_has_no_affected_actions() {
        let result = super::ExecutionResult::cancelled_before_start();

        assert_eq!(result.state, ExecutionState::Cancelled);
        assert!(result.completed.is_empty());
        assert!(result.incomplete.is_empty());
        assert!(result.not_started.is_empty());
    }

    #[test]
    fn only_completed_failed_and_cancelled_results_are_terminal() {
        assert!(super::ExecutionResult::failed_before_start().is_terminal());
        assert!(super::ExecutionResult::cancelled_before_start().is_terminal());
        assert!(
            !super::ExecutionResult {
                state: ExecutionState::Copying,
                completed: Vec::new(),
                incomplete: Vec::new(),
                not_started: Vec::new(),
            }
            .is_terminal()
        );
    }
}
