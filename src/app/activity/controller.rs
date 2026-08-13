use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc, time::Duration};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use uuid::Uuid;

use crate::{
    ActivityRow, AppWindow,
    app::activity::presentation::ActivityPresentation,
    capabilities::{DesktopNotification, DesktopNotifier},
    operations::execution::OperationExecutor,
    operations::queue::background::BackgroundQueue,
    platform::notifications::PlatformDesktopNotifier,
};

/// Connects a launch-scoped background queue to Activity UI rows and actions.
pub(crate) fn configure<E: OperationExecutor + Send + Sync + 'static>(
    window: &AppWindow,
    queue: Arc<BackgroundQueue<E>>,
) {
    crate::app::activity::result_controller::configure(window, Arc::clone(&queue));
    let weak = window.as_weak();
    let show_queue = Arc::clone(&queue);
    window.on_show_activity(move || show(&weak, Arc::clone(&show_queue)));
    schedule_refresh(
        window.as_weak(),
        Arc::clone(&queue),
        Rc::new(RefCell::new(HashMap::new())),
    );

    let weak = window.as_weak();
    let cancel_queue = Arc::clone(&queue);
    window.on_cancel_activity(move |operation_id| {
        if let Ok(operation_id) = Uuid::parse_str(operation_id.as_str()) {
            let _ = cancel_queue.cancel(operation_id);
        }
        refresh(&weak, &cancel_queue);
    });

    let weak = window.as_weak();
    window.on_request_cancel_activity(move |operation_id, name| {
        if let Some(window) = weak.upgrade() {
            window.set_pending_cancel_activity_id(operation_id);
            window.set_pending_cancel_activity_name(name);
            window.set_page(12);
        }
    });

    let weak = window.as_weak();
    let confirm_queue = Arc::clone(&queue);
    window.on_confirm_cancel_activity(move || {
        let Some(window) = weak.upgrade() else { return };
        let operation_id = window.get_pending_cancel_activity_id();
        if let Ok(operation_id) = Uuid::parse_str(operation_id.as_str()) {
            let _ = confirm_queue.cancel(operation_id);
        }
        window.set_pending_cancel_activity_id(SharedString::default());
        window.set_pending_cancel_activity_name(SharedString::default());
        window.set_page(9);
        refresh(&weak, &confirm_queue);
    });

    let weak = window.as_weak();
    let dismiss_queue = Arc::clone(&queue);
    window.on_dismiss_cancel_activity(move || {
        if let Some(window) = weak.upgrade() {
            window.set_pending_cancel_activity_id(SharedString::default());
            window.set_pending_cancel_activity_name(SharedString::default());
            window.set_page(9);
        }
        refresh(&weak, &dismiss_queue);
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
    announced: Rc<RefCell<HashMap<Uuid, i32>>>,
) {
    slint::Timer::single_shot(Duration::from_millis(250), move || {
        if weak.upgrade().is_none() {
            return;
        }
        refresh(&weak, &queue);
        announce_activity(&queue, &announced);
        schedule_refresh(weak, queue, announced);
    });
}

#[cfg(not(target_os = "android"))]
fn announce_activity<E: OperationExecutor + Send + Sync + 'static>(
    queue: &BackgroundQueue<E>,
    announced: &Rc<RefCell<HashMap<Uuid, i32>>>,
) {
    let notifier = PlatformDesktopNotifier::new();
    for entry in queue.activity() {
        let marker = match entry.state {
            crate::operations::queue::QueueState::Running => 0,
            crate::operations::queue::QueueState::Completed => 101,
            crate::operations::queue::QueueState::Failed => 102,
            crate::operations::queue::QueueState::Cancelled => 103,
            crate::operations::queue::QueueState::Queued => continue,
        };
        if announced.borrow().get(&entry.operation_id) == Some(&marker) {
            continue;
        }
        announced.borrow_mut().insert(entry.operation_id, marker);
        let presentation = ActivityPresentation::from_entry(&entry);
        let title = format!("SyncPak: {}", presentation.status);
        let body = if entry.state == crate::operations::queue::QueueState::Running {
            format!("{} — operation started", presentation.title)
        } else {
            format!("{} — {}", presentation.title, presentation.result_summary)
        };
        let _ = notifier.show(&DesktopNotification {
            title: &title,
            body: &body,
        });
    }
}

#[cfg(target_os = "android")]
fn announce_activity<E: OperationExecutor + Send + Sync + 'static>(
    _queue: &BackgroundQueue<E>,
    _announced: &Rc<RefCell<HashMap<Uuid, i32>>>,
) {
}

fn progress_percent(progress: &crate::operations::operation_progress::OperationProgress) -> i32 {
    if progress.total_bytes > 0 {
        ((progress.transferred_bytes.saturating_mul(100) / progress.total_bytes).min(100)) as i32
    } else if progress.total_items > 0 {
        ((progress.completed_items.saturating_mul(100) / progress.total_items).min(100)) as i32
    } else {
        0
    }
}

fn refresh<E: OperationExecutor + Send + Sync + 'static>(
    weak: &slint::Weak<AppWindow>,
    queue: &BackgroundQueue<E>,
) {
    let Some(window) = weak.upgrade() else { return };
    let activity = queue.activity();
    let active = activity
        .iter()
        .find(|entry| entry.state == crate::operations::queue::QueueState::Running);
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
    window.set_active_activity_cancelling(active.is_some_and(|entry| entry.cancellation_requested));
    let clearable_count = activity
        .iter()
        .filter(|entry| {
            matches!(
                entry.state,
                crate::operations::queue::QueueState::Completed
                    | crate::operations::queue::QueueState::Failed
                    | crate::operations::queue::QueueState::Cancelled
            )
        })
        .count();
    window.set_activity_clearable_count(clearable_count as i32);
    let rows = activity.into_iter().map(|entry| {
        let activity = ActivityPresentation::from_entry(&entry);
        let progress = entry.progress.as_ref();
        ActivityRow {
            id: activity.operation_id.into(),
            title: activity.title.into(),
            detail: activity.detail.into(),
            status: activity.status.into(),
            progress: activity.progress_summary.into(),
            current_path: progress
                .and_then(|progress| progress.current_path.clone())
                .unwrap_or_default()
                .into(),
            percent: progress.map_or(0, progress_percent),
            running: entry.state == crate::operations::queue::QueueState::Running,
            cancelling: entry.cancellation_requested,
            result: activity.result_summary.into(),
            can_cancel: activity.can_cancel,
            can_remove: activity.can_remove,
            can_view_result: activity.can_view_result,
        }
    });
    window.set_activity_rows(ModelRc::new(std::rc::Rc::new(VecModel::from_iter(rows))));
}
