use std::future::Future;

use crate::{
    cancellation::CancellationToken,
    configuration::SyncMode,
    destructive_confirmation::DestructiveConfirmation,
    execution::{ExecutionProgress, ExecutionResult, ExecutionState},
    inventory::RelativePath,
    planning::{Direction, Endpoint, PlannedAction, TransferPlan},
    transfer_progress::{TransferProgress, TransferProgressObserver},
};

pub use crate::mirror_execution_error::{MirrorActionError, MirrorExecutionError};

/// Applies one already-confirmed mirror action to its destination.
pub trait MirrorTransfer {
    type Error;

    fn copy(
        &self,
        direction: Direction,
        path: &RelativePath,
        overwrite: bool,
        cancellation: &CancellationToken,
        jitter_seed: u64,
    ) -> impl Future<Output = Result<(), Self::Error>>;

    fn delete(
        &self,
        direction: Direction,
        path: &RelativePath,
        cancellation: &CancellationToken,
    ) -> impl Future<Output = Result<(), Self::Error>>;
}

/// Executes a mirror plan only after its destructive actions have been confirmed.
pub async fn execute_confirmed_mirror<T: MirrorTransfer, O: TransferProgressObserver>(
    plan: &TransferPlan,
    confirmation: Option<&DestructiveConfirmation>,
    transfer: &T,
    cancellation: &CancellationToken,
    observer: &O,
    jitter_seed: u64,
) -> Result<ExecutionResult, MirrorExecutionError<T::Error>> {
    if plan.mode() != SyncMode::Mirror {
        return Err(MirrorExecutionError::NotMirrorPlan);
    }
    if plan.requires_confirmation() {
        confirmation
            .ok_or(MirrorExecutionError::ConfirmationRequired)?
            .verify(plan)
            .map_err(MirrorExecutionError::Confirmation)?;
    }
    let actions = ordered_actions(plan.actions());
    let total_actions = actions.len();
    let mut progress = ExecutionProgress::new(actions);
    let mut completed_actions = 0;
    loop {
        if cancellation.is_cancelled() {
            let result = progress.cancel();
            observer.on_progress(&event(
                ExecutionState::Cancelled,
                completed_actions,
                total_actions,
                None,
            ));
            return Ok(result);
        }
        let Some(action) = progress.start_next().cloned() else {
            let result = progress.finish();
            observer.on_progress(&event(
                ExecutionState::Completed,
                completed_actions,
                total_actions,
                None,
            ));
            return Ok(result);
        };
        observer.on_progress(&event(
            ExecutionState::Copying,
            completed_actions,
            total_actions,
            Some(action.clone()),
        ));
        if let Err(error) = apply_action(
            plan.direction(),
            &action,
            transfer,
            cancellation,
            jitter_seed.wrapping_add(completed_actions as u64),
        )
        .await
        {
            observer.on_progress(&event(
                ExecutionState::Failed,
                completed_actions,
                total_actions,
                Some(action.clone()),
            ));
            return Err(MirrorExecutionError::Action {
                error,
                result: progress.fail(),
            });
        }
        progress.complete_current();
        completed_actions += 1;
    }
}

async fn apply_action<T: MirrorTransfer>(
    direction: Direction,
    action: &PlannedAction,
    transfer: &T,
    cancellation: &CancellationToken,
    jitter_seed: u64,
) -> Result<(), MirrorActionError<T::Error>> {
    match action {
        PlannedAction::Copy {
            path,
            from: Endpoint::Source,
            to: Endpoint::Destination,
        } => transfer
            .copy(direction, path, false, cancellation, jitter_seed)
            .await
            .map_err(MirrorActionError::Transfer),
        PlannedAction::Overwrite {
            path,
            from: Endpoint::Source,
            to: Endpoint::Destination,
        } => transfer
            .copy(direction, path, true, cancellation, jitter_seed)
            .await
            .map_err(MirrorActionError::Transfer),
        PlannedAction::Delete {
            path,
            from: Endpoint::Destination,
        } => transfer
            .delete(direction, path, cancellation)
            .await
            .map_err(MirrorActionError::Transfer),
        _ => Err(MirrorActionError::Unsupported(action.clone())),
    }
}

fn ordered_actions(actions: &[PlannedAction]) -> Vec<PlannedAction> {
    let mut result = actions
        .iter()
        .filter(|action| !matches!(action, PlannedAction::Delete { .. }))
        .cloned()
        .collect::<Vec<_>>();
    let mut deletes = actions
        .iter()
        .filter_map(|action| match action {
            PlannedAction::Delete { path, .. } => {
                Some((path.as_str().matches('/').count(), path.as_str(), action))
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    deletes.sort_by(|left, right| right.0.cmp(&left.0).then_with(|| left.1.cmp(right.1)));
    result.extend(deletes.into_iter().map(|(_, _, action)| action.clone()));
    result
}

fn event(
    state: ExecutionState,
    completed: usize,
    total: usize,
    action: Option<PlannedAction>,
) -> TransferProgress {
    TransferProgress {
        state,
        completed_actions: completed,
        total_actions: total,
        current_action: action,
    }
}

#[cfg(test)]
#[path = "mirror_execution_tests.rs"]
mod tests;
