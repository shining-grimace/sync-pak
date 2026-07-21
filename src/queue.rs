use std::collections::VecDeque;

use uuid::Uuid;

use crate::{
    activity_snapshot::ActivitySnapshot,
    execution::{ExecutionResult, ExecutionState},
    operation_progress::OperationProgress,
    planning::OperationPlan,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
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
    pub snapshot: ActivitySnapshot,
    pub state: QueueState,
    pub progress: Option<OperationProgress>,
    pub result: Option<ExecutionResult>,
}

#[derive(Default)]
pub struct OperationQueue {
    entries: VecDeque<QueueEntry>,
}

impl OperationQueue {
    pub fn push(&mut self, plan: OperationPlan, snapshot: ActivitySnapshot) -> Uuid {
        let operation_id = Uuid::new_v4();
        self.entries.push_back(QueueEntry {
            operation_id,
            plan,
            snapshot,
            state: QueueState::Queued,
            progress: None,
            result: None,
        });
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
        true
    }

    /// Cancels a queued operation without starting it, retaining it in activity history.
    pub fn cancel_queued(&mut self, operation_id: Uuid) -> bool {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.operation_id == operation_id && entry.state == QueueState::Queued)
        else {
            return false;
        };
        entry.state = QueueState::Cancelled;
        entry.result = Some(ExecutionResult::cancelled_before_start());
        true
    }

    /// Removes queued work entirely, as requested from the Activity list.
    pub fn remove_queued(&mut self, operation_id: Uuid) -> bool {
        let Some(index) = self.entries.iter().position(|entry| {
            entry.operation_id == operation_id && entry.state == QueueState::Queued
        }) else {
            return false;
        };
        self.entries.remove(index);
        true
    }

    /// Removes queued work for a connection before its configuration is deleted.
    pub fn remove_queued_for_connection(&mut self, connection_id: &str) -> usize {
        let before = self.entries.len();
        self.entries.retain(|entry| {
            entry.state != QueueState::Queued || entry.plan.connection_id != connection_id
        });
        before - self.entries.len()
    }

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
        before - self.entries.len()
    }

    pub fn running(&self) -> Option<&QueueEntry> {
        self.entries
            .iter()
            .find(|entry| entry.state == QueueState::Running)
    }

    /// Returns whether there are no operations waiting to begin.
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
mod tests {
    use super::{OperationQueue, QueueState};
    use crate::{
        activity_snapshot::ActivitySnapshot,
        configuration::{ConnectionConfig, ConnectionId, ProviderId, SyncMode},
        execution::{ExecutionProgress, ExecutionState},
        planning::{Direction, OperationPlan},
    };

    fn snapshot(name: &str) -> ActivitySnapshot {
        ActivitySnapshot::from_connection(
            &ConnectionConfig {
                id: ConnectionId::new(),
                name: name.into(),
                provider_id: ProviderId::new(),
                bucket: "bucket".into(),
                remote_path: "path".into(),
                local_path: "/local".into(),
                mode: SyncMode::AddOnly,
                keep_last_archives: None,
            },
            "provider",
            Direction::Upload,
        )
    }

    #[test]
    fn queue_preserves_submission_order() {
        let mut queue = OperationQueue::default();
        queue.push(
            OperationPlan::new("first", SyncMode::AddOnly, Direction::Upload),
            snapshot("First"),
        );
        queue.push(
            OperationPlan::new("second", SyncMode::Mirror, Direction::Download),
            snapshot("Second"),
        );

        let first = queue.take_next().unwrap();
        assert_eq!(first.plan.connection_id, "first");
        assert!(queue.take_next().is_none());
        assert!(queue.finish(first.operation_id, ExecutionProgress::new([]).finish()));

        assert_eq!(queue.take_next().unwrap().plan.connection_id, "second");
        assert!(queue.is_empty());
    }

    #[test]
    fn terminal_results_remain_in_the_activity_list() {
        let mut queue = OperationQueue::default();
        queue.push(
            OperationPlan::new("connection", SyncMode::AddOnly, Direction::Upload),
            snapshot("Connection"),
        );
        let entry = queue.take_next().unwrap();
        let result = ExecutionProgress::new([]).cancel();

        assert!(queue.finish(entry.operation_id, result.clone()));

        let stored = queue.entries().next().unwrap();
        assert_eq!(stored.state, QueueState::Cancelled);
        assert_eq!(stored.result.as_ref(), Some(&result));
        assert_eq!(
            stored.result.as_ref().unwrap().state,
            ExecutionState::Cancelled
        );
    }

    #[test]
    fn queued_operation_can_be_cancelled_without_starting() {
        let mut queue = OperationQueue::default();
        let operation_id = queue.push(
            OperationPlan::new("connection", SyncMode::AddOnly, Direction::Upload),
            snapshot("Connection"),
        );

        assert!(queue.cancel_queued(operation_id));

        let entry = queue.entries().next().unwrap();
        assert_eq!(entry.state, QueueState::Cancelled);
        assert_eq!(
            entry.result.as_ref().unwrap().state,
            ExecutionState::Cancelled
        );
        assert!(queue.take_next().is_none());
    }

    #[test]
    fn deleting_a_connection_removes_only_its_queued_work() {
        let mut queue = OperationQueue::default();
        queue.push(
            OperationPlan::new("remove", SyncMode::AddOnly, Direction::Upload),
            snapshot("Remove"),
        );
        queue.push(
            OperationPlan::new("keep", SyncMode::AddOnly, Direction::Upload),
            snapshot("Keep"),
        );

        assert_eq!(queue.remove_queued_for_connection("remove"), 1);

        assert_eq!(queue.entries().count(), 1);
        assert_eq!(queue.take_next().unwrap().plan.connection_id, "keep");
    }

    #[test]
    fn activity_is_newest_first_and_can_clear_only_terminal_entries() {
        let mut queue = OperationQueue::default();
        let completed = queue.push(
            OperationPlan::new("old", SyncMode::AddOnly, Direction::Upload),
            snapshot("Old"),
        );
        let entry = queue.take_next().unwrap();
        assert_eq!(entry.operation_id, completed);
        assert!(queue.finish(entry.operation_id, ExecutionProgress::new([]).finish()));
        queue.push(
            OperationPlan::new("new", SyncMode::AddOnly, Direction::Upload),
            snapshot("New"),
        );

        assert_eq!(
            queue
                .activity()
                .map(|entry| entry.snapshot.connection_name.as_str())
                .collect::<Vec<_>>(),
            ["New", "Old"]
        );
        assert_eq!(queue.clear_completed(), 1);
        assert_eq!(queue.entries().count(), 1);
        assert_eq!(queue.running(), None);
    }

    #[test]
    fn queued_work_can_be_removed_without_creating_activity_history() {
        let mut queue = OperationQueue::default();
        let operation_id = queue.push(
            OperationPlan::new("connection", SyncMode::AddOnly, Direction::Upload),
            snapshot("Connection"),
        );

        assert!(queue.remove_queued(operation_id));
        assert!(queue.entries().next().is_none());
        assert!(!queue.remove_queued(operation_id));
    }

    #[test]
    fn running_work_retains_its_last_progress_snapshot_for_activity() {
        let mut queue = OperationQueue::default();
        queue.push(
            OperationPlan::new("connection", SyncMode::AddOnly, Direction::Upload),
            snapshot("Connection"),
        );
        let entry = queue.take_next().unwrap();
        let progress = crate::operation_progress::OperationProgress {
            completed_items: 1,
            total_items: 2,
            ..Default::default()
        };

        assert!(queue.update_progress(entry.operation_id, progress.clone()));
        assert_eq!(queue.entries().next().unwrap().progress, Some(progress));
    }
}
