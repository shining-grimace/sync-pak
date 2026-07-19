use std::sync::Mutex;

use crate::{
    cancellation::CancellationToken,
    capabilities::CapabilityError,
    execution::ExecutionState,
    inventory::RelativePath,
    planning::{Endpoint, PlannedAction},
    transfer_progress::{TransferProgress, TransferProgressObserver},
};

use super::{PlannedActionExecutor, execute_plan, execute_plan_with_progress};

struct Recorder {
    actions: Mutex<Vec<PlannedAction>>,
    cancellation: Option<CancellationToken>,
    failure: Option<CapabilityError>,
}

impl PlannedActionExecutor for Recorder {
    fn execute_action(&self, action: &PlannedAction) -> Result<(), CapabilityError> {
        self.actions.lock().unwrap().push(action.clone());
        if self.actions.lock().unwrap().len() == 1 {
            if let Some(cancellation) = &self.cancellation {
                cancellation.cancel();
            }
        }
        self.failure.map_or(Ok(()), Err)
    }
}

fn action(path: &str, delete: bool) -> PlannedAction {
    if delete {
        PlannedAction::Delete {
            path: RelativePath::new(path).unwrap(),
            from: Endpoint::Destination,
        }
    } else {
        PlannedAction::Copy {
            path: RelativePath::new(path).unwrap(),
            from: Endpoint::Source,
            to: Endpoint::Destination,
        }
    }
}

#[test]
fn completes_all_copies_before_a_delete() {
    let recorder = Recorder {
        actions: Mutex::new(Vec::new()),
        cancellation: None,
        failure: None,
    };

    execute_plan(
        &[action("delete-first", true), action("copy-second", false)],
        &recorder,
        &CancellationToken::default(),
    )
    .unwrap();

    assert_eq!(
        recorder.actions.lock().unwrap().as_slice(),
        [action("copy-second", false), action("delete-first", true)]
    );
}

#[test]
fn deletes_children_before_their_parent_directory() {
    let recorder = Recorder {
        actions: Mutex::new(Vec::new()),
        cancellation: None,
        failure: None,
    };

    execute_plan(
        &[action("folder", true), action("folder/child", true)],
        &recorder,
        &CancellationToken::default(),
    )
    .unwrap();

    assert_eq!(
        recorder.actions.lock().unwrap().as_slice(),
        [action("folder/child", true), action("folder", true)]
    );
}

#[test]
fn cancellation_stops_before_the_next_action() {
    let cancellation = CancellationToken::default();
    let recorder = Recorder {
        actions: Mutex::new(Vec::new()),
        cancellation: Some(cancellation.clone()),
        failure: None,
    };

    let result = execute_plan(
        &[action("first", false), action("second", false)],
        &recorder,
        &cancellation,
    )
    .unwrap();

    assert_eq!(result.state, ExecutionState::Cancelled);
    assert_eq!(result.completed, [action("first", false)]);
    assert_eq!(result.not_started, [action("second", false)]);
}

#[test]
fn failed_action_is_reported_as_incomplete() {
    let recorder = Recorder {
        actions: Mutex::new(Vec::new()),
        cancellation: None,
        failure: Some(CapabilityError::Unavailable),
    };

    let error = execute_plan(
        &[action("first", false), action("second", false)],
        &recorder,
        &CancellationToken::default(),
    )
    .unwrap_err();

    assert_eq!(error.error, CapabilityError::Unavailable);
    assert_eq!(error.result.state, ExecutionState::Failed);
    assert_eq!(error.result.incomplete, [action("first", false)]);
    assert_eq!(error.result.not_started, [action("second", false)]);
}

struct Observer(Mutex<Vec<TransferProgress>>);

impl TransferProgressObserver for Observer {
    fn on_progress(&self, progress: &TransferProgress) {
        self.0.lock().unwrap().push(progress.clone());
    }
}

#[test]
fn emits_action_and_terminal_progress() {
    let recorder = Recorder {
        actions: Mutex::new(Vec::new()),
        cancellation: None,
        failure: None,
    };
    let observer = Observer(Mutex::new(Vec::new()));

    execute_plan_with_progress(
        &[action("first", false)],
        &recorder,
        &CancellationToken::default(),
        &observer,
    )
    .unwrap();

    let events = observer.0.lock().unwrap();
    assert_eq!(events.len(), 2);
    assert_eq!(events[0].state, ExecutionState::Copying);
    assert_eq!(events[0].completed_actions, 0);
    assert_eq!(events[1].state, ExecutionState::Completed);
    assert_eq!(events[1].completed_actions, 1);
}
