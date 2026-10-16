use crate::{
    capabilities::CapabilityError,
    operations::{
        cancellation::CancellationToken,
        execution::{ExecutionResult, OperationExecutor},
        queue::QueueEntry,
        transfer::progress::TransferProgressObserver,
    },
};
use std::{collections::HashMap, path::PathBuf, sync::Mutex};

/// Runs confirmed S3 operations using the configuration and protected credentials for this launch.
pub struct S3OperationExecutor {
    configuration_path: PathBuf,
    cancellations: Mutex<HashMap<String, CancellationToken>>,
}

impl S3OperationExecutor {
    pub fn new(configuration_path: PathBuf) -> Self {
        Self {
            configuration_path,
            cancellations: Mutex::new(HashMap::new()),
        }
    }

    fn cancellation_for(&self, connection_id: &str) -> CancellationToken {
        self.cancellations
            .lock()
            .expect("cancellation mutex poisoned")
            .entry(connection_id.into())
            .or_default()
            .clone()
    }

    fn clear_cancellation(&self, connection_id: &str) {
        self.cancellations
            .lock()
            .expect("cancellation mutex poisoned")
            .remove(connection_id);
    }
}

impl OperationExecutor for S3OperationExecutor {
    fn execute(
        &self,
        entry: &QueueEntry,
        observer: &dyn TransferProgressObserver,
    ) -> Result<ExecutionResult, CapabilityError> {
        let request = entry
            .confirmed_operation
            .as_ref()
            .map(|c| c.request())
            .or(entry.automatic_request.as_ref())
            .ok_or(CapabilityError::InvalidReference)?;
        let connection_id = request.connection.id.as_str().to_owned();
        let cancellation = self.cancellation_for(&connection_id);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| CapabilityError::Unavailable)?;
        let result = runtime.block_on(super::run::execute_request(
            request,
            entry.confirmed_operation.as_ref(),
            &self.configuration_path,
            &cancellation,
            observer,
            entry.operation_id.as_u128() as u64,
        ));
        self.clear_cancellation(&connection_id);
        result
    }

    fn cancel(&self, connection_id: &str) -> Result<(), CapabilityError> {
        let active = crate::configuration::run_guard::active(&self.configuration_path)
            .is_some_and(|r| r.connection.id.as_str() == connection_id);
        let mut cancellations = self
            .cancellations
            .lock()
            .expect("cancellation mutex poisoned");
        let token = if active {
            cancellations
                .entry(connection_id.into())
                .or_default()
                .clone()
        } else {
            cancellations
                .get(connection_id)
                .cloned()
                .ok_or(CapabilityError::NotFound)?
        };
        token.cancel();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{capabilities::CapabilityError, operations::execution::OperationExecutor};

    use super::S3OperationExecutor;

    #[test]
    fn cancellation_requires_an_active_connection() {
        let executor = S3OperationExecutor::new(PathBuf::from("config.json"));

        assert_eq!(executor.cancel("missing"), Err(CapabilityError::NotFound));
    }
}
