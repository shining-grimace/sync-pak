mod cancellation;
mod lifecycle;
use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::JoinHandle,
};

use uuid::Uuid;

use crate::{
    capabilities::{BackgroundExecution, CapabilityError},
    operations::execution::OperationExecutor,
    operations::queue::snapshot::ActivitySnapshot,
    operations::queue::{OperationQueue, QueueEntry},
    preflight::reviewed_operation::ConfirmedOperation,
};

#[cfg(test)]
use crate::preflight::planning::OperationPlan;

/// Owns one background worker for this launch's in-memory operation queue.
///
/// The worker takes plans in submission order and never invokes the executor concurrently.
pub struct BackgroundQueue<E: OperationExecutor> {
    executor: Arc<E>,
    queue: Arc<(Mutex<OperationQueue>, Condvar)>,
    active_connection: Arc<Mutex<Option<String>>>,
    stopping: Arc<AtomicBool>,
    worker: Option<JoinHandle<()>>,
}

impl<E: OperationExecutor + Send + Sync + 'static> BackgroundQueue<E> {
    #[cfg(any(test, not(target_os = "android")))]
    pub fn new(executor: Arc<E>) -> Self {
        Self::new_with_background(executor, None)
    }

    /// Starts a foreground-service session around each active operation.
    #[cfg(any(test, target_os = "android"))]
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
        Self::with_queue(executor, background, OperationQueue::default())
    }

    /// Restores outcomes without restarting interrupted work. Usable by a future service host.
    pub fn persistent(
        executor: Arc<E>,
        background: Option<Arc<dyn BackgroundExecution + Send + Sync>>,
        path: std::path::PathBuf,
    ) -> std::io::Result<Self> {
        Ok(Self::with_queue(
            executor,
            background,
            OperationQueue::restore(path)?,
        ))
    }

    fn with_queue(
        executor: Arc<E>,
        background: Option<Arc<dyn BackgroundExecution + Send + Sync>>,
        initial: OperationQueue,
    ) -> Self {
        let queue = Arc::new((Mutex::new(initial), Condvar::new()));
        let stopping = Arc::new(AtomicBool::new(false));
        let active_connection = Arc::new(Mutex::new(None));
        #[cfg(target_os = "android")]
        lifecycle::install_android_cancellation(
            Arc::clone(&executor),
            Arc::clone(&active_connection),
        );
        let worker = Some(crate::operations::queue::worker::start(
            background.clone(),
            Arc::clone(&executor),
            Arc::clone(&queue),
            Arc::clone(&stopping),
            Arc::clone(&active_connection),
        ));
        Self {
            executor,
            queue,
            active_connection,
            stopping,
            worker,
        }
    }

    #[cfg(test)]
    pub fn enqueue(&self, plan: OperationPlan, snapshot: ActivitySnapshot) -> Uuid {
        let (queue, wake) = &*self.queue;
        let operation_id = queue
            .lock()
            .expect("queue mutex poisoned")
            .push(plan, snapshot);
        wake.notify_one();
        operation_id
    }

    pub(crate) fn submit_automatic(
        &self,
        request: crate::operations::request::RunRequest,
        snapshot: ActivitySnapshot,
        path: std::path::PathBuf,
    ) -> Uuid {
        let (queue, wake) = &*self.queue;
        let id = queue
            .lock()
            .expect("queue mutex poisoned")
            .push_automatic(request, snapshot, path);
        wake.notify_one();
        id
    }

    pub(crate) fn submit_reviewed(
        &self,
        operation: ConfirmedOperation,
        snapshot: ActivitySnapshot,
        path: std::path::PathBuf,
    ) -> Uuid {
        let (queue, wake) = &*self.queue;
        let mut queue = queue.lock().expect("queue mutex poisoned");
        let id = queue.push_confirmed(operation, snapshot);
        queue.set_configuration_path(id, path);
        drop(queue);
        wake.notify_one();
        id
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

    pub fn take_history_error(&self) -> Option<String> {
        self.queue
            .0
            .lock()
            .expect("queue mutex poisoned")
            .history_error
            .take()
    }

    pub fn clear_completed(&self) -> usize {
        let (queue, _) = &*self.queue;
        queue
            .lock()
            .expect("queue mutex poisoned")
            .clear_completed()
    }
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod notification_tests;
