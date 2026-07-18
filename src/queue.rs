use std::collections::VecDeque;

use crate::planning::OperationPlan;

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
    pub plan: OperationPlan,
    pub state: QueueState,
}

#[derive(Default)]
pub struct OperationQueue {
    entries: VecDeque<QueueEntry>,
}

impl OperationQueue {
    pub fn push(&mut self, plan: OperationPlan) {
        self.entries.push_back(QueueEntry {
            plan,
            state: QueueState::Queued,
        });
    }

    pub fn take_next(&mut self) -> Option<QueueEntry> {
        self.entries.pop_front().map(|mut entry| {
            entry.state = QueueState::Running;
            entry
        })
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::{OperationQueue, QueueState};
    use crate::{
        configuration::SyncMode,
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

        assert_eq!(queue.take_next().unwrap().plan.connection_id, "first");
        assert_eq!(queue.take_next().unwrap().state, QueueState::Running);
        assert!(queue.is_empty());
    }
}
