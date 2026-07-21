use uuid::Uuid;

use crate::{
    execution::ExecutionState,
    operation_progress::{OperationPhase, OperationProgress},
    planning::PlannedAction,
    transfer_progress::{TransferProgress, TransferProgressObserver},
};

/// Adapts transfer-executor events into the queue's UI-safe progress model.
pub struct QueueProgressObserver<F> {
    operation_id: Uuid,
    publish: F,
}

impl<F> QueueProgressObserver<F> {
    pub fn new(operation_id: Uuid, publish: F) -> Self {
        Self {
            operation_id,
            publish,
        }
    }
}

impl<F: Fn(Uuid, OperationProgress)> TransferProgressObserver for QueueProgressObserver<F> {
    fn on_progress(&self, progress: &TransferProgress) {
        (self.publish)(self.operation_id, from_transfer(progress));
    }
}

pub fn from_transfer(progress: &TransferProgress) -> OperationProgress {
    OperationProgress {
        phase: phase(progress),
        completed_items: progress.completed_actions,
        total_items: progress.total_actions,
        transferred_bytes: 0,
        total_bytes: 0,
        current_path: progress.current_action.as_ref().map(action_path),
        retry: None,
    }
}

fn phase(progress: &TransferProgress) -> OperationPhase {
    match progress.state {
        ExecutionState::Preparing => OperationPhase::Preparing,
        ExecutionState::Copying
            if matches!(
                progress.current_action.as_ref(),
                Some(PlannedAction::Delete { .. })
            ) =>
        {
            OperationPhase::Deleting
        }
        ExecutionState::Copying => OperationPhase::Copying,
        ExecutionState::Finalizing => OperationPhase::Finalizing,
        ExecutionState::Completed | ExecutionState::Failed | ExecutionState::Cancelled => {
            OperationPhase::CleaningUp
        }
    }
}

fn action_path(action: &PlannedAction) -> String {
    match action {
        PlannedAction::Copy { path, .. }
        | PlannedAction::Overwrite { path, .. }
        | PlannedAction::Delete { path, .. }
        | PlannedAction::SkipChanged { path } => path.as_str().into(),
        PlannedAction::CreateArchive { .. } => "Archive".into(),
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        execution::ExecutionState,
        inventory::RelativePath,
        operation_progress::OperationPhase,
        planning::{Endpoint, PlannedAction},
        transfer_progress::TransferProgress,
    };

    use super::from_transfer;

    #[test]
    fn delete_events_are_presented_as_the_deleting_phase() {
        let progress = from_transfer(&TransferProgress {
            state: ExecutionState::Copying,
            completed_actions: 2,
            total_actions: 3,
            current_action: Some(PlannedAction::Delete {
                path: RelativePath::new("old/photo.jpg").unwrap(),
                from: Endpoint::Destination,
            }),
        });

        assert_eq!(progress.phase, OperationPhase::Deleting);
        assert_eq!(progress.current_path.as_deref(), Some("old/photo.jpg"));
        assert_eq!(progress.completed_items, 2);
    }
}
