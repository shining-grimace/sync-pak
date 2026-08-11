#[cfg(test)]
use std::{error::Error, fmt};

pub mod delete;
pub mod download;
pub mod local_remote;
pub mod modes;
pub mod multipart;
pub mod multipart_file;
pub(crate) mod multipart_file_error;
pub mod paths;
#[cfg(test)]
pub mod preflight_add_only;
#[cfg(test)]
pub mod preflight_mirror;
pub mod progress;
pub mod upload;
pub(crate) mod upload_contents;
pub mod upload_strategy;

#[cfg(test)]
use crate::{
    capabilities::CapabilityError,
    operations::cancellation::CancellationToken,
    operations::execution::{ExecutionProgress, ExecutionResult, ExecutionState},
    operations::transfer::progress::{
        NoopProgressObserver, TransferProgress, TransferProgressObserver,
    },
    preflight::planning::PlannedAction,
};

/// Performs one planned action against its already-resolved endpoints.
#[cfg(test)]
pub trait PlannedActionExecutor {
    fn execute_action(&self, action: &PlannedAction) -> Result<(), CapabilityError>;
}

/// Executes actions serially, ensuring all copies complete before any deletion begins.
#[cfg(test)]
pub fn execute_plan<T: PlannedActionExecutor>(
    actions: &[PlannedAction],
    executor: &T,
    cancellation: &CancellationToken,
) -> Result<ExecutionResult, TransferExecutionError> {
    execute_plan_with_progress(actions, executor, cancellation, &NoopProgressObserver)
}

/// Executes actions and reports snapshots at every action boundary.
#[cfg(test)]
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

#[cfg(test)]
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

#[cfg(test)]
fn copy_before_delete(actions: &[PlannedAction]) -> impl Iterator<Item = PlannedAction> {
    let copies = actions
        .iter()
        .filter(|action| !matches!(action, PlannedAction::Delete { .. }))
        .cloned()
        .collect::<Vec<_>>();
    let mut deletes = actions
        .iter()
        .filter_map(|action| match action {
            PlannedAction::Delete { path, .. } => Some((
                path.as_str().matches('/').count(),
                path.as_str().to_owned(),
                action.clone(),
            )),
            _ => None,
        })
        .collect::<Vec<_>>();
    deletes.sort_by(|(left_depth, left_path, _), (right_depth, right_path, _)| {
        right_depth
            .cmp(left_depth)
            .then_with(|| left_path.cmp(right_path))
    });
    copies
        .into_iter()
        .chain(deletes.into_iter().map(|(_, _, action)| action))
}

#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg(test)]
pub struct TransferExecutionError {
    pub error: CapabilityError,
    pub result: ExecutionResult,
}

#[cfg(test)]
impl fmt::Display for TransferExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "transfer execution failed: {}", self.error)
    }
}

#[cfg(test)]
impl Error for TransferExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

#[cfg(test)]
mod tests;
