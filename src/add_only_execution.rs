use std::{error::Error, fmt, future::Future};

use crate::{
    cancellation::CancellationToken,
    execution::{ExecutionProgress, ExecutionResult, ExecutionState},
    inventory::RelativePath,
    planning::{Direction, Endpoint, PlannedAction},
    transfer_progress::{TransferProgress, TransferProgressObserver},
};

/// Transfers one relative path in either direction for add-only operations.
pub trait AddOnlyTransfer {
    type Error;

    fn upload(
        &self,
        path: &RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    fn download(
        &self,
        path: &RelativePath,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> impl Future<Output = Result<(), Self::Error>>;
}

/// Executes the copy and skip actions of an add-only plan in their planned order.
pub async fn execute_add_only_actions<T: AddOnlyTransfer, O: TransferProgressObserver>(
    direction: Direction,
    actions: &[PlannedAction],
    transfer: &T,
    cancellation: &CancellationToken,
    observer: &O,
    jitter_seed: u64,
) -> Result<ExecutionResult, AddOnlyExecutionError<T::Error>> {
    let mut progress = ExecutionProgress::new(actions.iter().cloned());
    let total_actions = actions.len();
    let mut completed_actions = 0;
    loop {
        if cancellation.is_cancelled() {
            let result = progress.cancel();
            observer.on_progress(&progress_event(
                ExecutionState::Cancelled,
                completed_actions,
                total_actions,
                None,
            ));
            return Ok(result);
        }
        let Some(action) = progress.start_next().cloned() else {
            let result = progress.finish();
            observer.on_progress(&progress_event(
                ExecutionState::Completed,
                completed_actions,
                total_actions,
                None,
            ));
            return Ok(result);
        };
        observer.on_progress(&progress_event(
            ExecutionState::Copying,
            completed_actions,
            total_actions,
            Some(action.clone()),
        ));
        let result = execute_action(
            direction,
            &action,
            transfer,
            cancellation,
            jitter_seed.wrapping_add(completed_actions as u64),
        )
        .await;
        if let Err(error) = result {
            observer.on_progress(&progress_event(
                ExecutionState::Failed,
                completed_actions,
                total_actions,
                Some(action.clone()),
            ));
            return Err(AddOnlyExecutionError {
                error,
                result: progress.fail(),
            });
        }
        progress.complete_current();
        completed_actions += 1;
    }
}

async fn execute_action<T: AddOnlyTransfer>(
    direction: Direction,
    action: &PlannedAction,
    transfer: &T,
    cancellation: &CancellationToken,
    jitter_seed: u64,
) -> Result<(), AddOnlyActionError<T::Error>> {
    match action {
        PlannedAction::SkipChanged { .. } => Ok(()),
        PlannedAction::Copy { path, from, to } => match (direction, from, to) {
            (Direction::Upload, Endpoint::Source, Endpoint::Destination)
            | (Direction::BothWays, Endpoint::Source, Endpoint::Destination) => transfer
                .upload(path, cancellation, jitter_seed)
                .await
                .map_err(AddOnlyActionError::Transfer),
            (Direction::Download, Endpoint::Source, Endpoint::Destination)
            | (Direction::BothWays, Endpoint::Destination, Endpoint::Source) => transfer
                .download(path, cancellation, jitter_seed)
                .await
                .map_err(AddOnlyActionError::Transfer),
            _ => Err(AddOnlyActionError::Unsupported(action.clone())),
        },
        _ => Err(AddOnlyActionError::Unsupported(action.clone())),
    }
}

fn progress_event(
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

#[derive(Debug)]
pub struct AddOnlyExecutionError<E> {
    pub error: AddOnlyActionError<E>,
    pub result: ExecutionResult,
}

#[derive(Debug)]
pub enum AddOnlyActionError<E> {
    Transfer(E),
    Unsupported(PlannedAction),
}

impl<E: fmt::Display> fmt::Display for AddOnlyExecutionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "add-only execution failed: {}", self.error)
    }
}

impl<E: Error + 'static> Error for AddOnlyExecutionError<E> {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match &self.error {
            AddOnlyActionError::Transfer(error) => Some(error),
            AddOnlyActionError::Unsupported(_) => None,
        }
    }
}

impl<E: fmt::Display> fmt::Display for AddOnlyActionError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Transfer(error) => error.fmt(formatter),
            Self::Unsupported(action) => {
                write!(formatter, "unsupported add-only action: {action:?}")
            }
        }
    }
}

#[cfg(test)]
#[path = "add_only_execution_tests.rs"]
mod tests;
