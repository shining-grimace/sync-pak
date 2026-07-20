use std::{sync::Mutex, thread, time::Duration};

use crate::{
    activity_snapshot::ActivitySnapshot,
    configuration::{ConnectionConfig, ConnectionId, ProviderId, SyncMode},
    execution::{ExecutionProgress, OperationExecutor},
    planning::{Direction, OperationPlan},
    queue::QueueState,
};

use super::BackgroundQueue;

struct Executor(Mutex<Vec<String>>);

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
