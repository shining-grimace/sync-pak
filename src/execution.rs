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
}
