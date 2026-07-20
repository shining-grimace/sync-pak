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
};

pub(crate) fn start<E: OperationExecutor + Send + Sync + 'static>(
    background: Option<Arc<dyn BackgroundExecution + Send + Sync>>,
    executor: Arc<E>,
    shared: Arc<(Mutex<OperationQueue>, Condvar)>,
    stopping: Arc<AtomicBool>,
) -> JoinHandle<()> {
    thread::spawn(move || {
        loop {
            let Some(entry) = next_entry(&shared, &stopping) else {
                return;
            };
            let (queue, _) = &*shared;
            let _ = queue.lock().expect("queue mutex poisoned").finish(
                entry.operation_id,
                execute(&*executor, background.as_deref(), &entry),
            );
        }
    })
}

fn execute<E: OperationExecutor>(
    executor: &E,
    background: Option<&(dyn BackgroundExecution + Send + Sync)>,
    entry: &QueueEntry,
) -> ExecutionResult {
    let foreground = background.map_or(Ok(()), |background| {
        background.start(&entry.snapshot.connection_name)
    });
    let result = match foreground {
        Ok(()) => executor
            .execute(&entry.plan)
            .unwrap_or_else(|_| ExecutionResult::failed_before_start()),
        Err(_) => ExecutionResult::failed_before_start(),
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
