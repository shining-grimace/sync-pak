use crate::{
    AppWindow,
    operations::{
        execution::OperationExecutor, queue::background::BackgroundQueue,
        run_coordinator::RunCoordinator,
    },
};
use slint::ComponentHandle;
use std::{path::PathBuf, sync::Arc};

pub(crate) fn configure<E: OperationExecutor + Send + Sync + 'static>(
    window: &AppWindow,
    queue: Arc<BackgroundQueue<E>>,
    path: PathBuf,
) {
    let coordinator = RunCoordinator::new(queue, path);
    let weak = window.as_weak();
    window.on_start_automatic_connection(move |id| {
        let Some(window) = weak.upgrade() else { return };
        match coordinator.start_automatic(id.as_str()) {
            Ok(_) => {
                window.set_operation_busy(true);
                window.set_busy_connection_id(id);
            }
            Err(error) => window.set_status_message(error.to_string().into()),
        }
    });
}
