use std::collections::VecDeque;

use uuid::Uuid;

use crate::{
    execution::{ExecutionResult, ExecutionState},
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
    pub state: QueueState,
    pub result: Option<ExecutionResult>,
}

#[derive(Default)]
pub struct OperationQueue {
    entries: VecDeque<QueueEntry>,
}

impl OperationQueue {
    pub fn push(&mut self, plan: OperationPlan) -> Uuid {
        let operation_id = Uuid::new_v4();
        self.entries.push_back(QueueEntry {
            operation_id,
            plan,
            state: QueueState::Queued,
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
        Some(entry.clone())
    }

    /// Stores an immutable terminal result on its activity entry.
    pub fn finish(&mut self, operation_id: Uuid, result: ExecutionResult) -> bool {
        let Some(entry) = self
            .entries
            .iter_mut()
            .find(|entry| entry.operation_id == operation_id && entry.state == QueueState::Running)
        else {
            return false;
        };
        entry.state = queue_state(result.state);
        entry.result = Some(result);
        true
    }

    pub fn entries(&self) -> impl Iterator<Item = &QueueEntry> {
        self.entries.iter()
    }

    /// Returns whether there are no operations waiting to begin.
    pub fn is_empty(&self) -> bool {
        !self
            .entries
            .iter()
            .any(|entry| entry.state == QueueState::Queued)
    }
}

fn queue_state(state: ExecutionState) -> QueueState {
    match state {
        ExecutionState::Cancelled => QueueState::Cancelled,
        ExecutionState::Failed => QueueState::Failed,
        ExecutionState::Preparing
        | ExecutionState::Copying
        | ExecutionState::Finalizing
        | ExecutionState::Completed => QueueState::Completed,
    }
}

#[cfg(test)]
mod tests {
    use super::{OperationQueue, QueueState};
    use crate::{
        configuration::SyncMode,
        execution::{ExecutionProgress, ExecutionState},
        planning::{Direction, OperationPlan},
    };

    #[test]
    fn queue_preserves_submission_order() {
        let mut queue = OperationQueue::default();
        queue.push(OperationPlan::new(
            "first",
            SyncMode::AddOnly,
            Direction::Upload,
        ));
        queue.push(OperationPlan::new(
            "second",
            SyncMode::Mirror,
            Direction::Download,
        ));

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
        queue.push(OperationPlan::new(
            "connection",
            SyncMode::AddOnly,
            Direction::Upload,
        ));
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
}
