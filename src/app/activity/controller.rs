use super::refresh::refresh;
use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::Arc, time::Duration};

use slint::{ComponentHandle, SharedString};
use uuid::Uuid;

use crate::{
    AppWindow,
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
    let announced = queue
        .activity()
        .into_iter()
        .filter_map(|entry| marker(entry.state).map(|marker| (entry.operation_id, marker)))
        .collect();
    schedule_refresh(
        window.as_weak(),
        Arc::clone(&queue),
        Rc::new(RefCell::new(announced)),
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
        if let Some(window) = weak.upgrade() {
            announce_activity(&window, &queue, &announced);
            #[cfg(target_os = "android")]
            crate::platform::android::result_navigation::show_requested(&window);
        }
        schedule_refresh(weak, queue, announced);
    });
}

fn announce_activity<E: OperationExecutor + Send + Sync + 'static>(
    window: &AppWindow,
    queue: &BackgroundQueue<E>,
    announced: &Rc<RefCell<HashMap<Uuid, i32>>>,
) {
    let notifier = PlatformDesktopNotifier::new();
    for entry in queue.activity() {
        let Some(marker) = marker(entry.state) else {
            continue;
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
        if entry.result.is_some() {
            window.invoke_queue_notification(
                body.clone().into(),
                if entry.state == crate::operations::queue::QueueState::Completed {
                    1
                } else {
                    0
                },
                0,
            );
        }
        let _ = notifier.show(&DesktopNotification {
            title: &title,
            body: &body,
        });
    }
}

fn marker(state: crate::operations::queue::QueueState) -> Option<i32> {
    use crate::operations::queue::QueueState;
    match state {
        QueueState::Running => Some(0),
        QueueState::Completed => Some(101),
        QueueState::Failed => Some(102),
        QueueState::Cancelled => Some(103),
        QueueState::Queued => None,
    }
}
