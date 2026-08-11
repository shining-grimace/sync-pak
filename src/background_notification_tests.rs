use std::{
    sync::{Arc, Mutex},
    thread,
    time::Duration,
};

use crate::{
    activity_snapshot::ActivitySnapshot,
    capabilities::BackgroundExecution,
    configuration::{ConnectionConfig, ConnectionId, ProviderId, SyncMode},
    execution::{ExecutionProgress, OperationExecutor},
    planning::{Direction, Endpoint, OperationPlan, PlannedAction},
    queue::{QueueEntry, QueueState},
    transfer_progress::TransferProgress,
};

use super::BackgroundQueue;

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

    fn update(
        &self,
        _: &str,
        progress: &crate::operation_progress::OperationProgress,
    ) -> Result<(), crate::CapabilityError> {
        self.0.lock().unwrap().push(format!(
            "update:{}:{}",
            progress.completed_items, progress.total_items
        ));
        Ok(())
    }

    fn stop(&self) -> Result<(), crate::CapabilityError> {
        self.0.lock().unwrap().push("stop".into());
        Ok(())
    }
}

struct ProgressExecutor(bool);

impl OperationExecutor for ProgressExecutor {
    fn execute(
        &self,
        _: &QueueEntry,
        observer: &dyn crate::transfer_progress::TransferProgressObserver,
    ) -> Result<crate::execution::ExecutionResult, crate::CapabilityError> {
        if self.0 {
            let action = PlannedAction::CreateArchive {
                from: Endpoint::Source,
                to: Endpoint::Destination,
            };
            observer.on_progress(&TransferProgress {
                state: crate::execution::ExecutionState::Copying,
                completed_actions: 1,
                total_actions: 3,
                current_action: Some(action.clone()),
            });
            observer.on_progress(&TransferProgress {
                state: crate::execution::ExecutionState::Completed,
                completed_actions: 2,
                total_actions: 3,
                current_action: Some(action),
            });
        }
        Ok(ExecutionProgress::new([]).finish())
    }

    fn cancel(&self, _: &str) -> Result<(), crate::CapabilityError> {
        Ok(())
    }
}

#[test]
fn foreground_execution_wraps_each_active_operation() {
    assert_eq!(execute(false), ["start:First", "stop"]);
}

#[test]
fn foreground_execution_updates_the_existing_notification_for_active_items() {
    assert_eq!(execute(true), ["start:First", "update:1:3", "stop"]);
}

fn execute(with_progress: bool) -> Vec<String> {
    let foreground = Arc::new(Foreground::default());
    let queue = BackgroundQueue::with_background_execution(
        Arc::new(ProgressExecutor(with_progress)),
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
    foreground.0.lock().unwrap().clone()
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
            verified: false,
        },
        "provider",
        Direction::Upload,
    )
}
