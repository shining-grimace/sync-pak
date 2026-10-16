use std::sync::Arc;

use slint::ComponentHandle;

use crate::{
    AppWindow, operations::execution::OperationExecutor,
    operations::queue::background::BackgroundQueue,
};

/// Submits the exact review visible in the preflight screen to the launch queue.
pub(crate) fn configure<E: OperationExecutor + Send + Sync + 'static>(
    window: &AppWindow,
    queue: Arc<BackgroundQueue<E>>,
    configuration_path: std::path::PathBuf,
) {
    let coordinator =
        crate::operations::run_coordinator::RunCoordinator::new(queue, configuration_path);
    let weak = window.as_weak();
    window.on_start_preflight_operation(move || {
        let Some(window) = weak.upgrade() else { return };
        let acknowledged = window.get_preflight_mirror_confirmed();
        let Ok(confirmed) = crate::app::preflight::controller::take_confirmed(acknowledged) else {
            window.set_status_message(
                "This review is no longer current. Refresh it before starting.".into(),
            );
            return;
        };
        if let Err(error) = coordinator.start_reviewed(confirmed) {
            window.set_status_message(error.to_string().into());
            return;
        }
        crate::app::preflight::controller::invalidate(&window);
        window.set_status_message("Operation queued. It will start shortly.".into());
        window.invoke_show_activity();
    });
}
