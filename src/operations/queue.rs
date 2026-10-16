use std::collections::VecDeque;

pub mod background;
mod cancellation;
pub mod history;
pub mod progress_observer;
#[cfg(test)]
pub mod runner;
pub mod snapshot;
mod submission;
pub(crate) mod worker;

use uuid::Uuid;

use crate::{
    operations::execution::{ExecutionResult, ExecutionState},
    operations::operation_progress::OperationProgress,
    operations::queue::snapshot::ActivitySnapshot,
    preflight::planning::OperationPlan,
    preflight::reviewed_operation::ConfirmedOperation,
};

#[cfg(test)]
use crate::operations::operation_progress::RetryStatus;

#[derive(Clone, Copy, Debug, Eq, PartialEq, serde::Serialize, serde::Deserialize)]
pub enum QueueState {
    Queued,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueueEntry {
    pub operation_id: Uuid,
    pub plan: OperationPlan,
    pub confirmed_operation: Option<ConfirmedOperation>,
    pub automatic_request: Option<crate::operations::request::RunRequest>,
    pub configuration_path: Option<std::path::PathBuf>,
    pub snapshot: ActivitySnapshot,
    pub state: QueueState,
    pub progress: Option<OperationProgress>,
    pub result: Option<ExecutionResult>,
    pub cancellation_requested: bool,
}

#[derive(Default)]
pub struct OperationQueue {
    entries: VecDeque<QueueEntry>,
    history_path: Option<std::path::PathBuf>,
    pub history_error: Option<String>,
}

impl OperationQueue {
    #[cfg(test)]
    pub fn push(&mut self, plan: OperationPlan, snapshot: ActivitySnapshot) -> Uuid {
        self.push_entry(plan, snapshot, None)
    }

    /// Queues the exact reviewed operation that was confirmed before execution.
    pub fn push_confirmed(
        &mut self,
        confirmed_operation: ConfirmedOperation,
        snapshot: ActivitySnapshot,
    ) -> Uuid {
        let plan = confirmed_operation.operation_plan();
        self.push_entry(plan, snapshot, Some(confirmed_operation))
    }

    fn push_entry(
        &mut self,
        plan: OperationPlan,
        snapshot: ActivitySnapshot,
        confirmed_operation: Option<ConfirmedOperation>,
    ) -> Uuid {
        let operation_id = Uuid::new_v4();
        self.entries.push_back(QueueEntry {
            operation_id,
            plan,
            confirmed_operation,
            automatic_request: None,
            configuration_path: None,
            snapshot,
            state: QueueState::Queued,
            progress: None,
            result: None,
            cancellation_requested: false,
        });
        self.persist();
        operation_id
    }

    /// Starts the oldest queued operation after any active operation has finished.
    pub fn take_next(&mut self) -> Option<QueueEntry> {
        if self
            .entries
            .iter()
            .any(|entry| entry.state == QueueState::Running)
        {
            return None;
        }
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.state == QueueState::Queued)?;
        entry.state = QueueState::Running;
        entry.progress = Some(OperationProgress::default());
        Some(entry.clone())
    }

    /// Replaces the active operation's non-secret progress snapshot.
    pub fn update_progress(&mut self, operation_id: Uuid, progress: OperationProgress) -> bool {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.operation_id == operation_id && entry.state == QueueState::Running)
        else {
            return false;
        };
        entry.progress = Some(progress);
        true
    }

    /// Adds retry status while retaining the latest phase, counts, and current path.
    #[cfg(test)]
    pub fn update_retry(&mut self, operation_id: Uuid, retry: RetryStatus) -> bool {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.operation_id == operation_id && entry.state == QueueState::Running)
        else {
            return false;
        };
        entry.progress.get_or_insert_default().retry = Some(retry);
        true
    }

    /// Stores an immutable terminal result on its activity entry.
    pub fn finish(&mut self, operation_id: Uuid, result: ExecutionResult) -> bool {
        let Some(state) = terminal_queue_state(result.state) else {
            return false;
        };
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.operation_id == operation_id && entry.state == QueueState::Running)
        else {
            return false;
        };
        entry.state = state;
        entry.result = Some(result);
        self.persist();
        true
    }

    #[cfg(test)]
    pub fn entries(&self) -> impl Iterator<Item = &QueueEntry> {
        self.entries.iter()
    }

    /// Returns activity entries newest first, without exposing mutable queue state.
    pub fn activity(&self) -> impl Iterator<Item = &QueueEntry> {
        self.entries.iter().rev()
    }

    /// Removes only terminal activity entries, retaining active and queued work.
    pub fn clear_completed(&mut self) -> usize {
        let before = self.entries.len();
        self.entries.retain(|entry| {
            !matches!(
                entry.state,
                QueueState::Completed | QueueState::Failed | QueueState::Cancelled
            )
        });
        self.persist();
        before - self.entries.len()
    }

    pub fn running(&self) -> Option<&QueueEntry> {
        self.entries
            .iter()
            .find(|entry| entry.state == QueueState::Running)
    }

    /// Returns whether there are no operations waiting to begin.
    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        !self
            .entries
            .iter()
            .any(|entry| entry.state == QueueState::Queued)
    }
}

fn terminal_queue_state(state: ExecutionState) -> Option<QueueState> {
    match state {
        ExecutionState::Cancelled => Some(QueueState::Cancelled),
        ExecutionState::Failed => Some(QueueState::Failed),
        ExecutionState::Completed => Some(QueueState::Completed),
        ExecutionState::Preparing | ExecutionState::Copying | ExecutionState::Finalizing => None,
    }
}

#[cfg(test)]
mod tests;
