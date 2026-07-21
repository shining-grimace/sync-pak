use std::{sync::Arc, time::Duration};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use uuid::Uuid;

use crate::{
    ActivityRow, AppWindow, activity_presentation::ActivityPresentation,
    background_queue::BackgroundQueue, execution::OperationExecutor,
};

/// Connects a launch-scoped background queue to Activity UI rows and actions.
pub(crate) fn configure<E: OperationExecutor + Send + Sync + 'static>(
    window: &AppWindow,
    queue: Arc<BackgroundQueue<E>>,
) {
    let weak = window.as_weak();
    let show_queue = Arc::clone(&queue);
    window.on_show_activity(move || show(&weak, Arc::clone(&show_queue)));

    let weak = window.as_weak();
    let cancel_queue = Arc::clone(&queue);
    window.on_cancel_activity(move |operation_id| {
        if let Ok(operation_id) = Uuid::parse_str(operation_id.as_str()) {
            let _ = cancel_queue.cancel(operation_id);
        }
        refresh(&weak, &cancel_queue);
    });

    let weak = window.as_weak();
    let remove_queue = Arc::clone(&queue);
    window.on_remove_queued_activity(move |operation_id| {
        if let Ok(operation_id) = Uuid::parse_str(operation_id.as_str()) {
            remove_queue.remove_queued(operation_id);
        }
        refresh(&weak, &remove_queue);
    });

    let weak = window.as_weak();
    window.on_clear_completed_activity(move || {
        queue.clear_completed();
        refresh(&weak, &queue);
    });
}

fn show<E: OperationExecutor + Send + Sync + 'static>(
    weak: &slint::Weak<AppWindow>,
    queue: Arc<BackgroundQueue<E>>,
) {
    let Some(window) = weak.upgrade() else { return };
    window.set_status_message(SharedString::default());
    window.set_page(9);
    refresh(weak, &queue);
    schedule_refresh(weak.clone(), queue);
}

fn schedule_refresh<E: OperationExecutor + Send + Sync + 'static>(
    weak: slint::Weak<AppWindow>,
    queue: Arc<BackgroundQueue<E>>,
) {
    slint::Timer::single_shot(Duration::from_millis(250), move || {
        let Some(window) = weak.upgrade() else { return };
        if window.get_page() != 9 {
            return;
        }
        refresh(&weak, &queue);
        schedule_refresh(weak, queue);
    });
}

fn refresh<E: OperationExecutor + Send + Sync + 'static>(
    weak: &slint::Weak<AppWindow>,
    queue: &BackgroundQueue<E>,
) {
    let Some(window) = weak.upgrade() else { return };
    let rows = queue.activity().into_iter().map(|entry| {
        let activity = ActivityPresentation::from_entry(&entry);
        ActivityRow {
            id: activity.operation_id.into(),
            title: activity.title.into(),
            detail: activity.detail.into(),
            status: activity.status.into(),
            progress: activity.progress_summary.into(),
            result: activity.result_summary.into(),
            can_cancel: activity.can_cancel,
            can_remove: activity.can_remove,
        }
    });
    window.set_activity_rows(ModelRc::new(std::rc::Rc::new(VecModel::from_iter(rows))));
}
