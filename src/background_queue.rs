use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};

use uuid::Uuid;

use crate::{
    activity_snapshot::ActivitySnapshot,
    capabilities::{BackgroundExecution, CapabilityError},
    execution::OperationExecutor,
    operation_progress::OperationProgress,
    planning::OperationPlan,
    queue::{OperationQueue, QueueEntry},
};

/// Owns one background worker for this launch's in-memory operation queue.
///
/// The worker takes plans in submission order and never invokes the executor concurrently.
pub struct BackgroundQueue<E> {
    executor: Arc<E>,
    queue: Arc<(Mutex<OperationQueue>, Condvar)>,
    stopping: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl<E: OperationExecutor + Send + Sync + 'static> BackgroundQueue<E> {
    pub fn new(executor: Arc<E>) -> Self {
        Self::new_with_background(executor, None)
    }

    /// Starts a foreground-service session around each active operation.
    pub fn with_background_execution(
        executor: Arc<E>,
        background: Arc<dyn BackgroundExecution + Send + Sync>,
    ) -> Self {
        Self::new_with_background(executor, Some(background))
    }

    fn new_with_background(
        executor: Arc<E>,
        background: Option<Arc<dyn BackgroundExecution + Send + Sync>>,
    ) -> Self {
        let queue = Arc::new((Mutex::new(OperationQueue::default()), Condvar::new()));
        let stopping = Arc::new(AtomicBool::new(false));
        let active_connection = Arc::new(Mutex::new(None));
        #[cfg(target_os = "android")]
        install_android_cancellation(Arc::clone(&executor), Arc::clone(&active_connection));
        let worker = Some(crate::background_worker::start(
            background.clone(),
            Arc::clone(&executor),
            Arc::clone(&queue),
            Arc::clone(&stopping),
            active_connection,
        ));
        Self {
            executor,
            queue,
            stopping,
            worker,
        }
    }

    pub fn enqueue(&self, plan: OperationPlan, snapshot: ActivitySnapshot) -> Uuid {
        let (queue, wake) = &*self.queue;
        let operation_id = queue
            .lock()
            .expect("queue mutex poisoned")
            .push(plan, snapshot);
        wake.notify_one();
        operation_id
    }

    /// Requests cancellation of queued work immediately, or of the active executor.
    pub fn cancel(&self, operation_id: Uuid) -> Result<bool, CapabilityError> {
        let connection_id = {
            let (queue, _) = &*self.queue;
            let mut queue = queue.lock().expect("queue mutex poisoned");
            if queue.cancel_queued(operation_id) {
                return Ok(true);
            }
            queue
                .running()
                .filter(|entry| entry.operation_id == operation_id)
                .map(|entry| entry.plan.connection_id.clone())
        };
        match connection_id {
            Some(connection_id) => self.executor.cancel(&connection_id).map(|()| true),
            None => Ok(false),
        }
    }

    /// Removes waiting work without recording a cancelled result.
    pub fn remove_queued(&self, operation_id: Uuid) -> bool {
        let (queue, _) = &*self.queue;
        queue
            .lock()
            .expect("queue mutex poisoned")
            .remove_queued(operation_id)
    }

    /// Publishes the active operation's non-secret progress for UI consumers.
    pub fn update_progress(&self, operation_id: Uuid, progress: OperationProgress) -> bool {
        let (queue, _) = &*self.queue;
        queue
            .lock()
            .expect("queue mutex poisoned")
            .update_progress(operation_id, progress)
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

    pub fn activity(&self) -> Vec<QueueEntry> {
        let (queue, _) = &*self.queue;
        queue
            .lock()
            .expect("queue mutex poisoned")
            .activity()
            .cloned()
            .collect()
    }

    pub fn clear_completed(&self) -> usize {
        let (queue, _) = &*self.queue;
        queue
            .lock()
            .expect("queue mutex poisoned")
            .clear_completed()
    }
}

impl<E: OperationExecutor + Send + Sync + 'static>
    crate::operation_cancellation::ConnectionOperationCanceller for BackgroundQueue<E>
{
    fn cancel_for_connection(&self, connection_id: &str) -> Result<usize, CapabilityError> {
        Self::cancel_for_connection(self, connection_id)
    }
}

impl<E> Drop for BackgroundQueue<E> {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        self.queue.1.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
        #[cfg(target_os = "android")]
        let _ = crate::android_foreground_execution::clear_cancel_handler();
    }
}

#[cfg(target_os = "android")]
fn install_android_cancellation<E: OperationExecutor + Send + Sync + 'static>(
    executor: Arc<E>,
    active_connection: Arc<Mutex<Option<String>>>,
) {
    let handler = Arc::new(move || {
        let connection_id = active_connection
            .lock()
            .ok()
            .and_then(|connection| connection.clone());
        if let Some(connection_id) = connection_id {
            let _ = executor.cancel(&connection_id);
        }
    });
    let _ = crate::android_foreground_execution::set_cancel_handler(handler);
}

#[cfg(test)]
#[path = "background_queue_tests.rs"]
mod tests;
