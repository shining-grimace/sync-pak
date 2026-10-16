//! One process-owned queue shared by the service, widgets and app screens.
use crate::{
    configuration::{ConfigStore, ConfigurationError},
    operations::{queue::background::BackgroundQueue, run_coordinator::RunCoordinator},
    providers::s3::operations::executor::S3OperationExecutor,
};
use std::sync::{Arc, Mutex};
pub type Queue = BackgroundQueue<S3OperationExecutor>;
static QUEUE: Mutex<Option<Arc<Queue>>> = Mutex::new(None);
pub fn queue() -> Result<Arc<Queue>, ConfigurationError> {
    let mut shared = QUEUE.lock().expect("Android runtime mutex poisoned");
    if let Some(queue) = shared.as_ref() {
        return Ok(Arc::clone(queue));
    }
    let config = ConfigStore::for_current_platform()?;
    let executor = Arc::new(S3OperationExecutor::new(config.path().into()));
    let background: Arc<dyn crate::capabilities::BackgroundExecution + Send + Sync> =
        Arc::new(crate::platform::PlatformBackgroundExecution);
    let queue = match BackgroundQueue::persistent(
        Arc::clone(&executor),
        Some(Arc::clone(&background)),
        config.path().with_file_name("activity.json"),
    ) {
        Ok(queue) => queue,
        Err(_) => {
            let queue = BackgroundQueue::with_background_execution(executor, background);
            queue.report_history_error("Activity history could not be loaded. New results are available for this session only.");
            queue
        }
    };
    let queue = Arc::new(queue);
    *shared = Some(Arc::clone(&queue));
    Ok(queue)
}
pub fn start(id: &str) -> Result<(), ConfigurationError> {
    let config = ConfigStore::for_current_platform()?;
    RunCoordinator::new(queue()?, config.path().into())
        .start_automatic(id)
        .map(|_| ())
}
