use std::{
    sync::{
        Arc, Condvar, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread::{self, JoinHandle},
};

use crate::{
    capabilities::BackgroundExecution,
    execution::{ExecutionResult, OperationExecutor},
    queue::{OperationQueue, QueueEntry},
    queue_progress_observer::QueueProgressObserver,
};

pub(crate) fn start<E: OperationExecutor + Send + Sync + 'static>(
    background: Option<Arc<dyn BackgroundExecution + Send + Sync>>,
    executor: Arc<E>,
    shared: Arc<(Mutex<OperationQueue>, Condvar)>,
    stopping: Arc<AtomicBool>,
    active_connection: Arc<Mutex<Option<String>>>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        loop {
            let Some(entry) = next_entry(&shared, &stopping) else {
                return;
            };
            *active_connection
                .lock()
                .expect("active connection mutex poisoned") =
                Some(entry.plan.connection_id.clone());
            // Progress callbacks update the queue, so execution must not hold its mutex.
            let result = execute(&*executor, background.as_deref(), &entry, &shared);
            let (queue, _) = &*shared;
            let _ = queue
                .lock()
                .expect("queue mutex poisoned")
                .finish(entry.operation_id, result);
            *active_connection
                .lock()
                .expect("active connection mutex poisoned") = None;
        }
    })
}

fn execute<E: OperationExecutor>(
    executor: &E,
    background: Option<&(dyn BackgroundExecution + Send + Sync)>,
    entry: &QueueEntry,
    shared: &(Mutex<OperationQueue>, Condvar),
) -> ExecutionResult {
    let foreground = background.map_or(Ok(()), |background| {
        background.start(&entry.snapshot.connection_name)
    });
    let result = match foreground {
        Ok(()) => {
            let observer =
                QueueProgressObserver::new(entry.operation_id, |operation_id, progress| {
                    let (queue, _) = shared;
                    let _ = queue
                        .lock()
                        .expect("queue mutex poisoned")
                        .update_progress(operation_id, progress);
                });
            executor.execute(entry, &observer).unwrap_or_else(|error| {
                ExecutionResult::failed_before_start_with_message(error.to_string())
            })
        }
        Err(error) => ExecutionResult::failed_before_start_with_message(error.to_string()),
    };
    if foreground.is_ok()
        && let Some(background) = background
    {
        let _ = background.stop();
    }
    if result.is_terminal() {
        result
    } else {
        ExecutionResult::failed_before_start()
    }
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
