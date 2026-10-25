use super::{OperationQueue, QueueState};
use crate::{
    configuration::{ConnectionConfig, ConnectionId, ProviderId, SyncMode},
    operations::execution::{ExecutionProgress, ExecutionState},
    operations::queue::snapshot::ActivitySnapshot,
    preflight::planning::{Direction, OperationPlan},
};

fn snapshot(name: &str) -> ActivitySnapshot {
    ActivitySnapshot::from_connection(
        &ConnectionConfig {
            last_run: None,
            transport: None,
            id: ConnectionId::new(),
            name: name.into(),
            provider_id: ProviderId::new(),
            bucket: "bucket".into(),
            remote_path: "path".into(),
            local_path: "/local".into(),
            mode: SyncMode::AddOnly,
            allow_upload: true,
            allow_download: true,
            show_preview: true,
            keep_last_archives: None,
            verified: false,
        },
        "provider",
        Direction::Upload,
    )
}

#[test]
fn queue_preserves_submission_order() {
    let mut queue = OperationQueue::default();
    queue.push(
        OperationPlan::new("first", SyncMode::AddOnly, Direction::Upload),
        snapshot("First"),
    );
    queue.push(
        OperationPlan::new("second", SyncMode::Mirror, Direction::Download),
        snapshot("Second"),
    );

    let first = queue.take_next().unwrap();
    assert_eq!(first.plan.connection_id, "first");
    assert!(queue.take_next().is_none());
    assert!(queue.finish(first.operation_id, ExecutionProgress::new([]).finish()));

    assert_eq!(queue.take_next().unwrap().plan.connection_id, "second");
    assert!(queue.is_empty());
}

#[test]
fn terminal_results_remain_in_the_activity_list() {
    let mut queue = OperationQueue::default();
    queue.push(
        OperationPlan::new("connection", SyncMode::AddOnly, Direction::Upload),
        snapshot("Connection"),
    );
    let entry = queue.take_next().unwrap();
    let result = ExecutionProgress::new([]).cancel();

    assert!(queue.finish(entry.operation_id, result.clone()));

    let stored = queue.entries().next().unwrap();
    assert_eq!(stored.state, QueueState::Cancelled);
    assert_eq!(stored.result.as_ref(), Some(&result));
    assert_eq!(
        stored.result.as_ref().unwrap().state,
        ExecutionState::Cancelled
    );
}

#[test]
fn queued_operation_can_be_cancelled_without_starting() {
    let mut queue = OperationQueue::default();
    let operation_id = queue.push(
        OperationPlan::new("connection", SyncMode::AddOnly, Direction::Upload),
        snapshot("Connection"),
    );

    assert!(queue.cancel_queued(operation_id));

    let entry = queue.entries().next().unwrap();
    assert_eq!(entry.state, QueueState::Cancelled);
    assert_eq!(
        entry.result.as_ref().unwrap().state,
        ExecutionState::Cancelled
    );
    assert!(queue.take_next().is_none());
}

#[test]
fn deleting_a_connection_removes_only_its_queued_work() {
    let mut queue = OperationQueue::default();
    queue.push(
        OperationPlan::new("remove", SyncMode::AddOnly, Direction::Upload),
        snapshot("Remove"),
    );
    queue.push(
        OperationPlan::new("keep", SyncMode::AddOnly, Direction::Upload),
        snapshot("Keep"),
    );

    assert_eq!(queue.remove_queued_for_connection("remove"), 1);

    assert_eq!(queue.entries().count(), 1);
    assert_eq!(queue.take_next().unwrap().plan.connection_id, "keep");
}

#[test]
fn activity_is_newest_first_and_can_clear_only_terminal_entries() {
    let mut queue = OperationQueue::default();
    let completed = queue.push(
        OperationPlan::new("old", SyncMode::AddOnly, Direction::Upload),
        snapshot("Old"),
    );
    let entry = queue.take_next().unwrap();
    assert_eq!(entry.operation_id, completed);
    assert!(queue.finish(entry.operation_id, ExecutionProgress::new([]).finish()));
    queue.push(
        OperationPlan::new("new", SyncMode::AddOnly, Direction::Upload),
        snapshot("New"),
    );

    assert_eq!(
        queue
            .activity()
            .map(|entry| entry.snapshot.connection_name.as_str())
            .collect::<Vec<_>>(),
        ["New", "Old"]
    );
    assert_eq!(queue.clear_completed(), 1);
    assert_eq!(queue.entries().count(), 1);
    assert_eq!(queue.running(), None);
}

#[test]
fn queued_work_can_be_removed_without_creating_activity_history() {
    let mut queue = OperationQueue::default();
    let operation_id = queue.push(
        OperationPlan::new("connection", SyncMode::AddOnly, Direction::Upload),
        snapshot("Connection"),
    );

    assert!(queue.remove_queued(operation_id));
    assert!(queue.entries().next().is_none());
    assert!(!queue.remove_queued(operation_id));
}

#[test]
fn running_work_retains_its_last_progress_snapshot_for_activity() {
    let mut queue = OperationQueue::default();
    queue.push(
        OperationPlan::new("connection", SyncMode::AddOnly, Direction::Upload),
        snapshot("Connection"),
    );
    let entry = queue.take_next().unwrap();
    let progress = crate::operations::operation_progress::OperationProgress {
        completed_items: 1,
        total_items: 2,
        ..Default::default()
    };

    assert!(queue.update_progress(entry.operation_id, progress.clone()));
    assert_eq!(queue.entries().next().unwrap().progress, Some(progress));
}

#[test]
fn retry_status_preserves_the_existing_progress_snapshot() {
    let mut queue = OperationQueue::default();
    queue.push(
        OperationPlan::new("connection", SyncMode::AddOnly, Direction::Upload),
        snapshot("Connection"),
    );
    let entry = queue.take_next().unwrap();
    let retry = crate::operations::operation_progress::RetryStatus {
        next_attempt: 2,
        max_attempts: 4,
        delay_millis: 250,
    };

    assert!(queue.update_retry(entry.operation_id, retry));
    assert_eq!(
        queue
            .entries()
            .next()
            .unwrap()
            .progress
            .as_ref()
            .unwrap()
            .retry,
        Some(retry)
    );
}
