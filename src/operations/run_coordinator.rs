//! UI-independent admission shared by connection cards and future platform triggers.
use crate::{
    configuration::{ConfigStore, ConfigurationError, run_guard},
    operations::{
        execution::OperationExecutor,
        queue::{background::BackgroundQueue, snapshot::ActivitySnapshot},
        request::RunRequest,
    },
    preflight::reviewed_operation::ConfirmedOperation,
};
use std::{path::PathBuf, sync::Arc};
use uuid::Uuid;

pub struct RunCoordinator<E: OperationExecutor> {
    pub queue: Arc<BackgroundQueue<E>>,
    configuration_path: PathBuf,
}
impl<E: OperationExecutor + Send + Sync + 'static> RunCoordinator<E> {
    pub fn new(queue: Arc<BackgroundQueue<E>>, configuration_path: PathBuf) -> Self {
        Self {
            queue,
            configuration_path,
        }
    }
    /// Resolves saved policy without creating a window or collecting inventory on the caller.
    pub fn start_automatic(&self, connection_id: &str) -> Result<Uuid, ConfigurationError> {
        run_guard::admit(&self.configuration_path, |state| {
            let config = ConfigStore::at(self.configuration_path.clone()).load()?;
            let request = RunRequest::automatic(&config, connection_id)
                .map_err(|e| ConfigurationError::Portable(e.to_string()))?;
            state.reserve(&request)?;
            let snapshot = ActivitySnapshot::from_connection(
                &request.connection,
                &request.provider.name,
                request.direction,
            );
            Ok(self
                .queue
                .submit_automatic(request, snapshot, self.configuration_path.clone()))
        })
    }
    pub fn start_reviewed(
        &self,
        operation: ConfirmedOperation,
    ) -> Result<Uuid, ConfigurationError> {
        run_guard::admit(&self.configuration_path, |state| {
            let request = operation.request();
            let config = ConfigStore::at(self.configuration_path.clone()).load()?;
            let current =
                RunRequest::from_config(&config, request.connection.id.as_str(), request.direction)
                    .map_err(|e| ConfigurationError::Portable(e.to_string()))?;
            if current != *request {
                return Err(ConfigurationError::Portable(
                    "Connection settings changed. Run the connection again.".into(),
                ));
            }
            state.reserve(request)?;
            let snapshot = ActivitySnapshot::from_connection(
                &request.connection,
                &request.provider.name,
                request.direction,
            );
            Ok(self
                .queue
                .submit_reviewed(operation, snapshot, self.configuration_path.clone()))
        })
    }
}

#[cfg(test)]
mod tests;
