use std::{sync::Arc, time::Duration};

use slint::{ComponentHandle, SharedString};
use uuid::Uuid;

use crate::{
    AppWindow, background_queue::BackgroundQueue, execution::OperationExecutor, queue::QueueState,
};

/// Presents the current non-secret progress snapshot for one running queue entry.
pub(crate) fn configure<E: OperationExecutor + Send + Sync + 'static>(
    window: &AppWindow,
    queue: Arc<BackgroundQueue<E>>,
) {
    let weak = window.as_weak();
    let show_queue = Arc::clone(&queue);
    window.on_show_activity_progress(move |operation_id| {
        show(&weak, &show_queue, operation_id.as_str());
    });

    let weak = window.as_weak();
    window.on_dismiss_activity_progress(move || {
        if let Some(window) = weak.upgrade() {
            window.set_page(9);
        }
    });
}

fn show<E: OperationExecutor + Send + Sync + 'static>(
    weak: &slint::Weak<AppWindow>,
    queue: &Arc<BackgroundQueue<E>>,
    operation_id: &str,
) {
    let Some(window) = weak.upgrade() else { return };
    let Ok(operation_id) = Uuid::parse_str(operation_id) else {
        return;
    };
    if !refresh(&window, queue, operation_id) {
        window.set_status_message("This operation is no longer running.".into());
        window.set_page(9);
        return;
    }
    window.set_activity_progress_id(operation_id.to_string().into());
    window.set_status_message(SharedString::default());
    window.set_page(17);
    schedule_refresh(weak.clone(), Arc::clone(queue), operation_id);
}

fn schedule_refresh<E: OperationExecutor + Send + Sync + 'static>(
    weak: slint::Weak<AppWindow>,
    queue: Arc<BackgroundQueue<E>>,
    operation_id: Uuid,
) {
    slint::Timer::single_shot(Duration::from_millis(250), move || {
        let Some(window) = weak.upgrade() else { return };
        if window.get_page() != 17
            || window.get_activity_progress_id().as_str() != operation_id.to_string()
        {
            return;
        }
        if refresh(&window, &queue, operation_id) {
            schedule_refresh(weak, queue, operation_id);
        } else {
            window.set_activity_progress_id(SharedString::default());
            window.set_page(9);
        }
    });
}

fn refresh<E: OperationExecutor + Send + Sync + 'static>(
    window: &AppWindow,
    queue: &BackgroundQueue<E>,
    operation_id: Uuid,
) -> bool {
    let Some(entry) = queue
        .activity()
        .into_iter()
        .find(|entry| entry.operation_id == operation_id && entry.state == QueueState::Running)
    else {
        return false;
    };
    let progress = entry.progress.as_ref();
    window.set_activity_progress_title(entry.snapshot.connection_name.clone().into());
    window.set_activity_progress_phase(
        progress
            .map_or("Preparing", |progress| progress.phase.label())
            .into(),
    );
    window.set_activity_progress_summary(
        progress
            .map_or_else(
                || "Preparing".into(),
                crate::operation_progress::OperationProgress::summary,
            )
            .into(),
    );
    window.set_activity_progress_items(progress.map_or_else(
        || "Item progress is not available yet.".into(),
        |progress| {
            if progress.total_items == 0 {
                "Item progress is not available yet.".into()
            } else {
                format!(
                    "{} of {} items",
                    progress.completed_items, progress.total_items
                )
                .into()
            }
        },
    ));
    window.set_activity_progress_bytes(progress.map_or_else(
        || "Byte progress is not available yet.".into(),
        |progress| {
            if progress.total_bytes == 0 {
                "Byte progress is not available yet.".into()
            } else {
                format!(
                    "{} of {} bytes",
                    progress.transferred_bytes, progress.total_bytes
                )
                .into()
            }
        },
    ));
    window.set_activity_progress_path(
        progress
            .and_then(|progress| progress.current_path.as_deref())
            .unwrap_or("No file is active yet.")
            .into(),
    );
    true
}
