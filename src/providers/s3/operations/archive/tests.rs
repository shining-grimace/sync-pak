use crate::{operations::transfer::progress::NoopProgressObserver, preflight::planning::Endpoint};

use super::*;

fn archive_action() -> PlannedAction {
    PlannedAction::CreateArchive {
        from: Endpoint::Source,
        to: Endpoint::Destination,
    }
}

#[tokio::test]
async fn successful_storage_completes_the_archive_action() {
    let action = archive_action();
    let result = execute_action(
        action.clone(),
        &CancellationToken::default(),
        &NoopProgressObserver,
        async { true },
    )
    .await;

    assert_eq!(result.state, ExecutionState::Completed);
    assert_eq!(result.completed, [action]);
    assert!(result.not_started.is_empty());
    assert_eq!(result.summary(), "Archive created.");
}

#[tokio::test]
async fn storage_failure_is_incomplete_and_early_cancellation_is_not_started() {
    let action = archive_action();
    let cancellation = CancellationToken::default();
    let failed = execute_action(
        action.clone(),
        &cancellation,
        &NoopProgressObserver,
        async { false },
    )
    .await;
    assert_eq!(failed.state, ExecutionState::Failed);
    assert_eq!(failed.incomplete, [action.clone()]);
    assert!(failed.not_started.is_empty());

    cancellation.cancel();
    let cancelled = execute_action(
        action.clone(),
        &cancellation,
        &NoopProgressObserver,
        async { panic!("cancelled archive must not be stored") },
    )
    .await;
    assert_eq!(cancelled.state, ExecutionState::Cancelled);
    assert_eq!(cancelled.not_started, [action]);
}
