use super::*;

impl OperationQueue {
    /// Marks a running entry as cancelling while the executor stops at its next safe point.
    pub fn request_cancel(&mut self, operation_id: Uuid) -> Option<String> {
        let entry = self.entries.iter_mut().find(|entry| {
            entry.operation_id == operation_id && entry.state == QueueState::Running
        })?;
        entry.cancellation_requested = true;
        Some(entry.plan.connection_id.clone())
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
        self.persist();
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
        self.persist();
        true
    }

    /// Removes queued work for a connection before its configuration is deleted.
    pub fn remove_queued_for_connection(&mut self, connection_id: &str) -> usize {
        let before = self.entries.len();
        self.entries.retain(|entry| {
            entry.state != QueueState::Queued || entry.plan.connection_id != connection_id
        });
        self.persist();
        before - self.entries.len()
    }
}
