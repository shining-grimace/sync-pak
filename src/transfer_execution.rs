use std::{error::Error, fmt};

use crate::{
    cancellation::CancellationToken,
    capabilities::CapabilityError,
    execution::{ExecutionProgress, ExecutionResult},
    planning::PlannedAction,
};

/// Performs one planned action against its already-resolved endpoints.
pub trait PlannedActionExecutor {
    fn execute_action(&self, action: &PlannedAction) -> Result<(), CapabilityError>;
}

/// Executes actions serially, ensuring all copies complete before any deletion begins.
pub fn execute_plan<T: PlannedActionExecutor>(
    actions: &[PlannedAction],
    executor: &T,
    cancellation: &CancellationToken,
) -> Result<ExecutionResult, TransferExecutionError> {
    let mut progress = ExecutionProgress::new(copy_before_delete(actions));
    loop {
        if cancellation.is_cancelled() {
            return Ok(progress.cancel());
        }
        let Some(action) = progress.start_next().cloned() else {
            return Ok(progress.finish());
        };
        if let Err(error) = executor.execute_action(&action) {
            return Err(TransferExecutionError {
                error,
                result: progress.fail(),
            });
        }
        progress.complete_current();
    }
}

fn copy_before_delete(actions: &[PlannedAction]) -> impl Iterator<Item = PlannedAction> + '_ {
    actions
        .iter()
        .filter(|action| !matches!(action, PlannedAction::Delete { .. }))
        .chain(
            actions
                .iter()
                .filter(|action| matches!(action, PlannedAction::Delete { .. })),
        )
        .cloned()
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferExecutionError {
    pub error: CapabilityError,
    pub result: ExecutionResult,
}

impl fmt::Display for TransferExecutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "transfer execution failed: {}", self.error)
    }
}

impl Error for TransferExecutionError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.error)
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use crate::{
        cancellation::CancellationToken,
        capabilities::CapabilityError,
        execution::ExecutionState,
        inventory::RelativePath,
        planning::{Endpoint, PlannedAction},
    };

    use super::{PlannedActionExecutor, execute_plan};

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
        let actions = [action("delete-first", true), action("copy-second", false)];

        execute_plan(&actions, &recorder, &CancellationToken::default()).unwrap();

        assert_eq!(
            recorder.actions.lock().unwrap().as_slice(),
            [action("copy-second", false), action("delete-first", true)]
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
        let actions = [action("first", false), action("second", false)];

        let result = execute_plan(&actions, &recorder, &cancellation).unwrap();

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
        let actions = [action("first", false), action("second", false)];

        let error = execute_plan(&actions, &recorder, &CancellationToken::default()).unwrap_err();

        assert_eq!(error.error, CapabilityError::Unavailable);
        assert_eq!(error.result.state, ExecutionState::Failed);
        assert_eq!(error.result.incomplete, [action("first", false)]);
        assert_eq!(error.result.not_started, [action("second", false)]);
    }
}
