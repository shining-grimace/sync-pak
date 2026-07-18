use crate::{
    capabilities::CapabilityError,
    execution::{ExecutionResult, OperationExecutor},
    queue::OperationQueue,
};

/// Runs one queued operation at a time and records its terminal activity result.
pub struct QueueRunner<'a, E> {
    executor: &'a E,
}

impl<'a, E: OperationExecutor> QueueRunner<'a, E> {
    pub fn new(executor: &'a E) -> Self {
        Self { executor }
    }

    /// Runs the next queued operation, if no operation is already active.
    pub fn run_next(&self, queue: &mut OperationQueue) -> Result<bool, CapabilityError> {
        let Some(entry) = queue.take_next() else {
            return Ok(false);
        };
        match self.executor.execute(&entry.plan) {
            Ok(result) if result.is_terminal() => {
                if queue.finish(entry.operation_id, result) {
                    Ok(true)
                } else {
                    Err(CapabilityError::Unexpected)
                }
            }
            Ok(_) => {
                let _ = queue.finish(entry.operation_id, ExecutionResult::failed_before_start());
                Err(CapabilityError::Unexpected)
            }
            Err(error) => {
                let _ = queue.finish(entry.operation_id, ExecutionResult::failed_before_start());
                Err(error)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use crate::{
        activity_snapshot::ActivitySnapshot,
        capabilities::CapabilityError,
        configuration::{ConnectionConfig, ConnectionId, ProviderId, SyncMode},
        execution::{ExecutionProgress, ExecutionState, OperationExecutor},
        planning::{Direction, OperationPlan},
        queue::QueueState,
    };

    use super::QueueRunner;

    struct Executor {
        completed: Mutex<Vec<String>>,
    }

    impl OperationExecutor for Executor {
        fn execute(
            &self,
            plan: &OperationPlan,
        ) -> Result<crate::execution::ExecutionResult, CapabilityError> {
            self.completed
                .lock()
                .unwrap()
                .push(plan.connection_id.clone());
            Ok(ExecutionProgress::new([]).finish())
        }

        fn cancel(&self, _: &str) -> Result<(), CapabilityError> {
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
    fn runs_queued_operations_in_order_and_records_results() {
        let mut queue = crate::queue::OperationQueue::default();
        queue.push(
            OperationPlan::new("first", SyncMode::AddOnly, Direction::Upload),
            snapshot("First"),
        );
        queue.push(
            OperationPlan::new("second", SyncMode::AddOnly, Direction::Upload),
            snapshot("Second"),
        );
        let executor = Executor {
            completed: Mutex::new(Vec::new()),
        };
        let runner = QueueRunner::new(&executor);

        assert_eq!(runner.run_next(&mut queue), Ok(true));
        assert_eq!(runner.run_next(&mut queue), Ok(true));
        assert_eq!(runner.run_next(&mut queue), Ok(false));

        assert_eq!(
            executor.completed.lock().unwrap().as_slice(),
            ["first", "second"]
        );
        assert!(
            queue
                .entries()
                .all(|entry| entry.state == QueueState::Completed)
        );
        assert!(queue.entries().all(|entry| {
            entry.result.as_ref().map(|result| result.state) == Some(ExecutionState::Completed)
        }));
    }
}
