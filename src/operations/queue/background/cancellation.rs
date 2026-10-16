use super::*;

impl<E: OperationExecutor + Send + Sync + 'static> BackgroundQueue<E> {
    /// Requests cancellation of queued work immediately, or of the active executor.
    pub fn cancel(&self, operation_id: Uuid) -> Result<bool, CapabilityError> {
        let (connection_id, cancelled) = {
            let (queue, _) = &*self.queue;
            let mut queue = queue.lock().expect("queue mutex poisoned");
            let entry = queue
                .activity()
                .find(|e| e.operation_id == operation_id)
                .cloned();
            if queue.cancel_queued(operation_id) {
                (None, entry)
            } else {
                (queue.request_cancel(operation_id), None)
            }
        };
        if let Some(entry) = cancelled {
            release(&entry);
            return Ok(true);
        }
        match connection_id {
            Some(connection_id) => self.executor.cancel(&connection_id).map(|()| true),
            None => Ok(false),
        }
    }

    /// Removes waiting work without recording a cancelled result.
    pub fn remove_queued(&self, operation_id: Uuid) -> bool {
        let removed = {
            let (queue, _) = &*self.queue;
            let mut queue = queue.lock().expect("queue mutex poisoned");
            let entry = queue
                .activity()
                .find(|e| e.operation_id == operation_id)
                .cloned();
            queue.remove_queued(operation_id).then_some(entry).flatten()
        };
        if let Some(entry) = removed {
            release(&entry);
            true
        } else {
            false
        }
    }

    /// Cancels active work and removes queued work before its connection is deleted.
    pub fn cancel_for_connection(&self, connection_id: &str) -> Result<usize, CapabilityError> {
        let active = {
            let (queue, _) = &*self.queue;
            let mut queue = queue.lock().expect("queue mutex poisoned");
            let removed = queue.remove_queued_for_connection(connection_id);
            (
                removed,
                queue
                    .running()
                    .is_some_and(|entry| entry.plan.connection_id == connection_id),
            )
        };
        if active.1 {
            self.executor.cancel(connection_id)?;
        }
        Ok(active.0)
    }
}
impl<E: OperationExecutor + Send + Sync + 'static>
    crate::operations::operation_cancellation::ConnectionOperationCanceller for BackgroundQueue<E>
{
    fn cancel_for_connection(&self, connection_id: &str) -> Result<usize, CapabilityError> {
        Self::cancel_for_connection(self, connection_id)
    }
}

pub(super) fn release(entry: &QueueEntry) {
    if let Some(path) = &entry.configuration_path {
        crate::configuration::run_guard::release(path, &entry.plan.connection_id);
    }
}
