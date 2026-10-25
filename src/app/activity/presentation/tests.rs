use crate::{
    app::activity::presentation::ActivityPresentation,
    configuration::{ConnectionConfig, ConnectionId, ProviderId, SyncMode},
    inventory::RelativePath,
    operations::execution::ExecutionProgress,
    operations::queue::OperationQueue,
    operations::queue::snapshot::ActivitySnapshot,
    preflight::planning::{Direction, Endpoint, OperationPlan, PlannedAction},
};

fn snapshot() -> ActivitySnapshot {
    ActivitySnapshot::from_connection(
        &ConnectionConfig {
            last_run: None,
            transport: None,
            id: ConnectionId::new(),
            name: "Photos".into(),
            provider_id: ProviderId::new(),
            bucket: "backups".into(),
            remote_path: "phone".into(),
            local_path: "/pictures".into(),
            mode: SyncMode::AddOnly,
            allow_upload: true,
            allow_download: true,
            show_preview: true,
            keep_last_archives: None,
            verified: false,
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
    assert_eq!(
        presentation.detail,
        "Add-only · Upload · /pictures → R2 / backups / phone"
    );
    assert_eq!(presentation.status, "Completed");
    assert_eq!(presentation.progress_summary, "");
    assert_eq!(presentation.result_summary, "No changes were needed.");
    assert!(!presentation.can_cancel);
    assert!(!presentation.can_remove);
}

#[test]
fn identifies_archive_and_bidirectional_activity_context() {
    let mut queue = OperationQueue::default();
    let mut archive = snapshot();
    archive.mode = SyncMode::Archive;
    archive.direction = Direction::Download;
    queue.push(
        OperationPlan::new("connection", SyncMode::Archive, Direction::Download),
        archive,
    );

    let presentation = ActivityPresentation::from_entry(queue.entries().next().unwrap());

    assert!(presentation.detail.starts_with("Archive · Download ·"));
    assert_eq!(super::direction(Direction::BothWays), "Both ways");
}

#[test]
fn presents_a_queued_cancellation_as_not_started() {
    let mut queue = OperationQueue::default();
    let operation_id = queue.push(
        OperationPlan::new("connection", SyncMode::AddOnly, Direction::Upload),
        snapshot(),
    );
    assert!(queue.cancel_queued(operation_id));

    let presentation = ActivityPresentation::from_entry(queue.entries().next().unwrap());

    assert_eq!(presentation.status, "Cancelled");
    assert_eq!(presentation.progress_summary, "");
    assert_eq!(presentation.result_summary, "Cancelled before starting");
}

#[test]
fn presents_each_terminal_action_with_its_outcome() {
    let mut queue = OperationQueue::default();
    queue.push(
        OperationPlan::new("connection", SyncMode::AddOnly, Direction::Upload),
        snapshot(),
    );
    let entry = queue.take_next().unwrap();
    let action = PlannedAction::Copy {
        path: RelativePath::new("photos/new.jpg").unwrap(),
        from: Endpoint::Source,
        to: Endpoint::Destination,
    };
    let mut progress = ExecutionProgress::new([action]);
    assert!(progress.start_next().is_some());
    assert!(progress.complete_current());
    assert!(queue.finish(entry.operation_id, progress.finish()));

    let presentation = ActivityPresentation::from_entry(queue.entries().next().unwrap());

    assert_eq!(presentation.result_details, ["Completed: photos/new.jpg"]);
    assert!(presentation.can_view_result);
}

#[test]
fn presents_a_safe_failure_reason_for_work_that_never_started() {
    let mut queue = OperationQueue::default();
    queue.push(
        OperationPlan::new("connection", SyncMode::AddOnly, Direction::Upload),
        snapshot(),
    );
    let entry = queue.take_next().unwrap();
    assert!(queue.finish(
        entry.operation_id,
        crate::operations::execution::ExecutionResult::failed_before_start_with_message(
            "Files changed since this review. Refresh the review before starting the operation.",
        ),
    ));

    let presentation = ActivityPresentation::from_entry(queue.entries().next().unwrap());

    assert_eq!(
        presentation.result_summary,
        "Files changed since this review. Refresh the review before starting the operation."
    );
}

#[test]
fn skipped_files_are_counted_once_at_the_end_of_results() {
    let skip = |path| PlannedAction::SkipChanged {
        path: RelativePath::new(path).unwrap(),
    };
    let copied = PlannedAction::Copy {
        path: RelativePath::new("new.jpg").unwrap(),
        from: Endpoint::Source,
        to: Endpoint::Destination,
    };
    let mut progress =
        ExecutionProgress::new([skip("existing-a.jpg"), copied, skip("existing-b.jpg")]);
    while progress.start_next().is_some() {
        assert!(progress.complete_current());
    }
    let result = progress.finish();
    assert_eq!(result.summary(), "1 item completed · 2 files skipped");
    assert_eq!(
        super::result_details(&result),
        ["Completed: new.jpg", "2 files skipped"]
    );
}

#[test]
fn an_all_skipped_run_shows_a_summary_without_file_rows() {
    let mut progress = ExecutionProgress::new([PlannedAction::SkipChanged {
        path: RelativePath::new("existing.jpg").unwrap(),
    }]);
    progress.start_next();
    assert!(progress.complete_current());
    let result = progress.finish();
    assert_eq!(result.summary(), "0 items completed · 1 file skipped");
    assert_eq!(super::result_details(&result), ["1 file skipped"]);
}

#[test]
fn uncompleted_skip_actions_are_not_counted_as_skipped_files() {
    let mut progress = ExecutionProgress::new(["done", "failed", "pending"].map(|path| {
        PlannedAction::SkipChanged {
            path: RelativePath::new(path).unwrap(),
        }
    }));
    progress.start_next();
    assert!(progress.complete_current());
    progress.start_next();
    assert_eq!(
        super::result_details(&progress.fail()),
        [
            "Incomplete: failed",
            "Not started: pending",
            "1 file skipped",
        ]
    );
}
