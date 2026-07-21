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
    schedule_refresh(window.as_weak(), Arc::clone(&queue));

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
}

fn schedule_refresh<E: OperationExecutor + Send + Sync + 'static>(
    weak: slint::Weak<AppWindow>,
    queue: Arc<BackgroundQueue<E>>,
) {
    slint::Timer::single_shot(Duration::from_millis(250), move || {
        if weak.upgrade().is_none() {
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
    let activity = queue.activity();
    let active = activity
        .iter()
        .find(|entry| entry.state == crate::queue::QueueState::Running);
    window.set_active_activity_id(active.map_or_else(Default::default, |entry| {
        entry.operation_id.to_string().into()
    }));
    window.set_active_activity_title(active.map_or_else(Default::default, |entry| {
        entry.snapshot.connection_name.clone().into()
    }));
    window.set_active_activity_progress(
        active
            .and_then(|entry| entry.progress.as_ref())
            .map_or_else(Default::default, |progress| progress.summary().into()),
    );
    let rows = activity.into_iter().map(|entry| {
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
