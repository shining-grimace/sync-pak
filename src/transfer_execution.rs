use std::{error::Error, fmt};

use crate::{
    cancellation::CancellationToken,
    capabilities::CapabilityError,
    execution::{ExecutionProgress, ExecutionResult, ExecutionState},
    planning::PlannedAction,
    transfer_progress::{NoopProgressObserver, TransferProgress, TransferProgressObserver},
};

/// Performs one planned action against its already-resolved endpoints.
pub trait PlannedActionExecutor {
    fn execute_action(&self, action: &PlannedAction) -> Result<(), CapabilityError>;
}

/// Executes actions serially, ensuring all copies complete before any deletion begins.
pub fn execute_plan<T: PlannedActionExecutor>(
    actions: &[PlannedAction],
    executor: &T,
    cancellation: &CancellationToken,
) -> Result<ExecutionResult, TransferExecutionError> {
    execute_plan_with_progress(actions, executor, cancellation, &NoopProgressObserver)
}

/// Executes actions and reports snapshots at every action boundary.
pub fn execute_plan_with_progress<T: PlannedActionExecutor, O: TransferProgressObserver>(
    actions: &[PlannedAction],
    executor: &T,
    cancellation: &CancellationToken,
    observer: &O,
) -> Result<ExecutionResult, TransferExecutionError> {
    let mut progress = ExecutionProgress::new(copy_before_delete(actions));
    let total_actions = actions.len();
    let mut completed_actions = 0;
    loop {
        if cancellation.is_cancelled() {
            let result = progress.cancel();
            observer.on_progress(&snapshot(
                ExecutionState::Cancelled,
                completed_actions,
                total_actions,
                None,
            ));
            return Ok(result);
        }
        let Some(action) = progress.start_next().cloned() else {
            let result = progress.finish();
            observer.on_progress(&snapshot(
                ExecutionState::Completed,
                completed_actions,
                total_actions,
                None,
            ));
            return Ok(result);
        };
        observer.on_progress(&snapshot(
            ExecutionState::Copying,
            completed_actions,
            total_actions,
            Some(action.clone()),
        ));
        if let Err(error) = executor.execute_action(&action) {
            observer.on_progress(&snapshot(
                ExecutionState::Failed,
                completed_actions,
                total_actions,
                Some(action),
            ));
            return Err(TransferExecutionError {
                error,
                result: progress.fail(),
            });
        }
        progress.complete_current();
        completed_actions += 1;
    }
}

fn snapshot(
    state: ExecutionState,
    completed_actions: usize,
    total_actions: usize,
    current_action: Option<PlannedAction>,
) -> TransferProgress {
    TransferProgress {
        state,
        completed_actions,
        total_actions,
        current_action,
    }
}

fn copy_before_delete(actions: &[PlannedAction]) -> impl Iterator<Item = PlannedAction> + '_ {
    actions
        .iter()
        .filter(|action| !matches!(action, PlannedAction::Delete { .. }))
        .chain(
            actions
                .iter()
                .filter(|action| matches!(action, PlannedAction::Delete { .. })),
        )
        .cloned()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferExecutionError {
    pub error: CapabilityError,
    pub result: ExecutionResult,
}

impl fmt::Display for TransferExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "transfer execution failed: {}", self.error)
    }
}

impl Error for TransferExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

#[cfg(test)]
#[path = "transfer_execution_tests.rs"]
mod tests;
