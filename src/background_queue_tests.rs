use std::{
    sync::{
        Mutex,
        atomic::{AtomicBool, Ordering},
    },
    thread,
    time::Duration,
};

use crate::{
    activity_snapshot::ActivitySnapshot,
    capabilities::BackgroundExecution,
    configuration::{ConnectionConfig, ConnectionId, ProviderId, SyncMode},
    execution::{ExecutionProgress, OperationExecutor},
    planning::{Direction, OperationPlan},
    queue::QueueState,
};

use super::BackgroundQueue;

struct Executor(Mutex<Vec<String>>);

struct BlockingExecutor {
    cancelled: Mutex<Vec<String>>,
    started: AtomicBool,
}

#[derive(Default)]
struct Foreground(Mutex<Vec<String>>);

impl BackgroundExecution for Foreground {
    fn start(&self, connection_name: &str) -> Result<(), crate::CapabilityError> {
        self.0
            .lock()
            .unwrap()
            .push(format!("start:{connection_name}"));
        Ok(())
    }

    fn stop(&self) -> Result<(), crate::CapabilityError> {
        self.0.lock().unwrap().push("stop".into());
        Ok(())
    }
}

impl OperationExecutor for Executor {
    fn execute(
        &self,
        plan: &OperationPlan,
    ) -> Result<crate::execution::ExecutionResult, crate::CapabilityError> {
        self.0.lock().unwrap().push(plan.connection_id.clone());
        Ok(ExecutionProgress::new([]).finish())
    }

    fn cancel(&self, _: &str) -> Result<(), crate::CapabilityError> {
        Ok(())
    }
}

impl OperationExecutor for BlockingExecutor {
    fn execute(
        &self,
        plan: &OperationPlan,
    ) -> Result<crate::execution::ExecutionResult, crate::CapabilityError> {
        self.started.store(true, Ordering::Release);
        while !self
            .cancelled
            .lock()
            .unwrap()
            .iter()
            .any(|connection| connection == &plan.connection_id)
        {
            thread::sleep(Duration::from_millis(1));
        }
        Ok(ExecutionProgress::new([]).cancel())
    }

    fn cancel(&self, connection_id: &str) -> Result<(), crate::CapabilityError> {
        self.cancelled.lock().unwrap().push(connection_id.into());
        Ok(())
    }
}

fn snapshot(name: &str) -> ActivitySnapshot {
    ActivitySnapshot::from_connection(
        &ConnectionConfig {
            id: ConnectionId::new(),
            name: name.into(),
            provider_id: ProviderId::new(),
            bucket: "bucket".into(),
            remote_path: String::new(),
            local_path: "/local".into(),
            mode: SyncMode::AddOnly,
            keep_last_archives: None,
        },
        "provider",
        Direction::Upload,
    )
}

#[test]
fn worker_completes_operations_in_submission_order() {
    let executor = std::sync::Arc::new(Executor(Mutex::new(Vec::new())));
    let queue = BackgroundQueue::new(std::sync::Arc::clone(&executor));
    queue.enqueue(
        OperationPlan::new("first", SyncMode::AddOnly, Direction::Upload),
        snapshot("First"),
    );
    queue.enqueue(
        OperationPlan::new("second", SyncMode::AddOnly, Direction::Upload),
        snapshot("Second"),
    );
    for _ in 0..50 {
        let activity = queue.activity();
        if activity.len() == 2
            && activity
                .iter()
                .all(|entry| entry.state == QueueState::Completed)
        {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(executor.0.lock().unwrap().as_slice(), ["first", "second"]);
    assert!(
        queue
            .activity()
            .iter()
            .all(|entry| entry.state == QueueState::Completed)
    );
}

#[test]
fn foreground_execution_wraps_each_active_operation() {
    let executor = std::sync::Arc::new(Executor(Mutex::new(Vec::new())));
    let foreground = std::sync::Arc::new(Foreground::default());
    let queue = BackgroundQueue::with_background_execution(
        std::sync::Arc::clone(&executor),
        foreground.clone(),
    );
    queue.enqueue(
        OperationPlan::new("first", SyncMode::AddOnly, Direction::Upload),
        snapshot("First"),
    );
    for _ in 0..50 {
        if queue
            .activity()
            .iter()
            .all(|entry| entry.state == QueueState::Completed)
        {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }
    assert_eq!(
        foreground.0.lock().unwrap().as_slice(),
        ["start:First", "stop"]
    );
}

#[test]
fn deleting_a_connection_cancels_active_work_and_removes_its_pending_work() {
    let executor = std::sync::Arc::new(BlockingExecutor {
        cancelled: Mutex::new(Vec::new()),
        started: AtomicBool::new(false),
    });
    let queue = BackgroundQueue::new(std::sync::Arc::clone(&executor));
    queue.enqueue(
        OperationPlan::new("remove", SyncMode::AddOnly, Direction::Upload),
        snapshot("Remove"),
    );
    queue.enqueue(
        OperationPlan::new("remove", SyncMode::AddOnly, Direction::Download),
        snapshot("Remove"),
    );
    for _ in 0..50 {
        if executor.started.load(Ordering::Acquire) {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }

    assert!(executor.started.load(Ordering::Acquire));
    assert_eq!(queue.cancel_for_connection("remove"), Ok(1));
    for _ in 0..50 {
        if queue
            .activity()
            .iter()
            .all(|entry| entry.state == QueueState::Cancelled)
        {
            break;
        }
        thread::sleep(Duration::from_millis(2));
    }

    assert_eq!(executor.cancelled.lock().unwrap().as_slice(), ["remove"]);
    assert_eq!(queue.activity().len(), 1);
    assert_eq!(queue.activity()[0].state, QueueState::Cancelled);
}

#[test]
fn dropping_the_queue_cancels_active_work_before_waiting_for_the_worker() {
    let executor = std::sync::Arc::new(BlockingExecutor {
        cancelled: Mutex::new(Vec::new()),
        started: AtomicBool::new(false),
    });
    {
        let queue = BackgroundQueue::new(std::sync::Arc::clone(&executor));
        queue.enqueue(
            OperationPlan::new("active", SyncMode::AddOnly, Direction::Upload),
            snapshot("Active"),
        );
        for _ in 0..50 {
            if executor.started.load(Ordering::Acquire) {
                break;
            }
            thread::sleep(Duration::from_millis(2));
        }
        assert!(executor.started.load(Ordering::Acquire));
    }

    assert_eq!(executor.cancelled.lock().unwrap().as_slice(), ["active"]);
}
