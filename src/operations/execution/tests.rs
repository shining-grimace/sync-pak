use crate::{
    inventory::RelativePath,
    preflight::planning::{Endpoint, PlannedAction},
};

use super::{ExecutionProgress, ExecutionState};

fn actions() -> Vec<PlannedAction> {
    ["first", "second", "third"]
        .into_iter()
        .map(|path| PlannedAction::Copy {
            path: RelativePath::new(path).unwrap(),
            from: Endpoint::Source,
            to: Endpoint::Destination,
        })
        .collect()
}

#[test]
fn cancellation_keeps_completed_incomplete_and_not_started_actions_separate() {
    let mut progress = ExecutionProgress::new(actions());
    progress.start_next();
    assert!(progress.complete_current());
    progress.start_next();

    let result = progress.cancel();

    assert_eq!(result.state, ExecutionState::Cancelled);
    assert_eq!(result.completed.len(), 1);
    assert_eq!(result.incomplete.len(), 1);
    assert_eq!(result.not_started.len(), 1);
}

#[test]
fn completed_run_has_no_remaining_actions() {
    let mut progress = ExecutionProgress::new(actions());
    while progress.start_next().is_some() {
        assert!(progress.complete_current());
    }

    let result = progress.finish();

    assert_eq!(result.state, ExecutionState::Completed);
    assert!(result.incomplete.is_empty());
    assert!(result.not_started.is_empty());
    assert_eq!(result.completed.len(), 3);
}

#[test]
fn cancellation_before_start_has_no_affected_actions() {
    let result = super::ExecutionResult::cancelled_before_start();

    assert_eq!(result.state, ExecutionState::Cancelled);
    assert!(result.completed.is_empty());
    assert!(result.incomplete.is_empty());
    assert!(result.not_started.is_empty());
}

#[test]
fn only_completed_failed_and_cancelled_results_are_terminal() {
    assert!(super::ExecutionResult::failed_before_start().is_terminal());
    assert!(super::ExecutionResult::cancelled_before_start().is_terminal());
    assert!(
        !super::ExecutionResult {
            state: ExecutionState::Copying,
            failure_message: None,
            completed: Vec::new(),
            incomplete: Vec::new(),
            not_started: Vec::new(),
        }
        .is_terminal()
    );
}
