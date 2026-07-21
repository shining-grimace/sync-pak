use crate::{
    activity_presentation::ActivityPresentation,
    activity_snapshot::ActivitySnapshot,
    configuration::{ConnectionConfig, ConnectionId, ProviderId, SyncMode},
    execution::ExecutionProgress,
    planning::{Direction, OperationPlan},
    queue::OperationQueue,
};

fn snapshot() -> ActivitySnapshot {
    ActivitySnapshot::from_connection(
        &ConnectionConfig {
            id: ConnectionId::new(),
            name: "Photos".into(),
            provider_id: ProviderId::new(),
            bucket: "backups".into(),
            remote_path: "phone".into(),
            local_path: "/pictures".into(),
            mode: SyncMode::AddOnly,
            keep_last_archives: None,
        },
        "R2",
        Direction::Upload,
    )
}

#[test]
fn presents_terminal_results_from_the_immutable_activity_snapshot() {
    let mut queue = OperationQueue::default();
    queue.push(
        OperationPlan::new("connection", SyncMode::AddOnly, Direction::Upload),
        snapshot(),
    );
    let entry = queue.take_next().unwrap();
    assert!(queue.finish(entry.operation_id, ExecutionProgress::new([]).finish()));

    let presentation = ActivityPresentation::from_entry(queue.entries().next().unwrap());

    assert_eq!(presentation.title, "Photos");
    assert_eq!(presentation.detail, "/pictures → R2 / backups / phone");
    assert_eq!(presentation.status, "Completed");
    assert_eq!(presentation.progress_summary, "Preparing");
    assert_eq!(presentation.result_summary, "0 items completed");
    assert!(!presentation.can_cancel);
    assert!(!presentation.can_remove);
}
