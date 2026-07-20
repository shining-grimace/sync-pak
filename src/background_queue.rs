use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use uuid::Uuid;

use crate::{
    activity_snapshot::ActivitySnapshot,
    capabilities::CapabilityError,
    execution::{ExecutionResult, OperationExecutor},
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
        let queue = Arc::new((Mutex::new(OperationQueue::default()), Condvar::new()));
        let stopping = Arc::new(AtomicBool::new(false));
        let worker = Some(start_worker(
            Arc::clone(&executor),
            Arc::clone(&queue),
            Arc::clone(&stopping),
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

impl<E> Drop for BackgroundQueue<E> {
    fn drop(&mut self) {
        self.stopping.store(true, Ordering::Release);
        self.queue.1.notify_one();
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn start_worker<E: OperationExecutor + Send + Sync + 'static>(
    executor: Arc<E>,
    shared: Arc<(Mutex<OperationQueue>, Condvar)>,
    stopping: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        loop {
            let entry = next_entry(&shared, &stopping);
            let Some(entry) = entry else { return };
            let result = executor
                .execute(&entry.plan)
                .unwrap_or_else(|_| ExecutionResult::failed_before_start());
            let result = if result.is_terminal() {
                result
            } else {
                ExecutionResult::failed_before_start()
            };
            let (queue, _) = &*shared;
            let _ = queue
                .lock()
                .expect("queue mutex poisoned")
                .finish(entry.operation_id, result);
        }
    })
}

fn next_entry(
    shared: &(Mutex<OperationQueue>, Condvar),
    stopping: &AtomicBool,
) -> Option<QueueEntry> {
    let (queue, wake) = shared;
    let mut queue = queue.lock().expect("queue mutex poisoned");
    loop {
        if stopping.load(Ordering::Acquire) {
            return None;
        }
        if let Some(entry) = queue.take_next() {
            return Some(entry);
        }
        queue = wake.wait(queue).expect("queue mutex poisoned");
    }
}

#[cfg(test)]
#[path = "background_queue_tests.rs"]
mod tests;
