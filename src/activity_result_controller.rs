use std::sync::Arc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use uuid::Uuid;

use crate::{
    AppWindow, activity_presentation::ActivityPresentation, background_queue::BackgroundQueue,
    execution::OperationExecutor,
};

/// Shows immutable per-action outcomes retained by the launch-scoped Activity queue.
pub(crate) fn configure<E: OperationExecutor + Send + Sync + 'static>(
    window: &AppWindow,
    queue: Arc<BackgroundQueue<E>>,
) {
    let weak = window.as_weak();
    window.on_show_activity_result(move |operation_id| {
        show(&weak, &queue, operation_id.as_str());
    });

    let weak = window.as_weak();
    window.on_dismiss_activity_result(move || {
        if let Some(window) = weak.upgrade() {
            window.set_page(9);
        }
    });
}

fn show<E: OperationExecutor + Send + Sync + 'static>(
    weak: &slint::Weak<AppWindow>,
    queue: &BackgroundQueue<E>,
    operation_id: &str,
) {
    let Some(window) = weak.upgrade() else { return };
    let Ok(operation_id) = Uuid::parse_str(operation_id) else {
        return;
    };
    let Some(entry) = queue
        .activity()
        .into_iter()
        .find(|entry| entry.operation_id == operation_id)
    else {
        window.set_status_message("This activity result is no longer available.".into());
        window.set_page(9);
        return;
    };
    let activity = ActivityPresentation::from_entry(&entry);
    if !activity.can_view_result {
        return;
    }
    window.set_activity_result_title(activity.title.into());
    window.set_activity_result_status(activity.status.into());
    window.set_activity_result_summary(activity.result_summary.into());
    window.set_activity_result_details(ModelRc::new(std::rc::Rc::new(VecModel::from_iter(
        activity.result_details.into_iter().map(SharedString::from),
    ))));
    window.set_status_message(SharedString::default());
    window.set_page(16);
}
