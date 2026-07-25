use std::sync::Arc;

use slint::ComponentHandle;

use crate::{
    AppWindow, activity_snapshot::ActivitySnapshot, background_queue::BackgroundQueue,
    execution::OperationExecutor,
};

/// Submits the exact review visible in the preflight screen to the launch queue.
pub(crate) fn configure<E: OperationExecutor + Send + Sync + 'static>(
    window: &AppWindow,
    queue: Arc<BackgroundQueue<E>>,
) {
    let weak = window.as_weak();
    window.on_start_preflight_operation(move || {
        let Some(window) = weak.upgrade() else { return };
        let acknowledged = window.get_preflight_mirror_confirmed();
        let Ok(confirmed) = crate::preflight_controller::take_confirmed(acknowledged) else {
            window.set_status_message(
                "This review is no longer current. Refresh it before starting.".into(),
            );
            return;
        };
        let snapshot = ActivitySnapshot::from_connection(
            &confirmed.request().connection,
            &confirmed.request().provider.name,
            confirmed.request().direction,
        );
        queue.enqueue_confirmed(confirmed, snapshot);
        crate::preflight_controller::invalidate(&window);
        window.set_status_message("Operation queued. It will start shortly.".into());
        window.invoke_show_activity();
    });
}
