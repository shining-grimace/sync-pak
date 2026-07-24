use std::sync::Arc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};
use uuid::Uuid;

use crate::{
    AppWindow, activity_presentation::ActivityPresentation, background_queue::BackgroundQueue,
    execution::OperationExecutor, planning::Direction, queue::QueueState,
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

    let weak = window.as_weak();
    window.on_retry_activity_result(move || retry(&weak));
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
    window.set_activity_result_connection_id(entry.plan.connection_id.into());
    window.set_activity_result_retry_direction(direction_index(entry.plan.direction));
    window.set_activity_result_can_retry(retryable(entry.state));
    window.set_activity_result_details(ModelRc::new(std::rc::Rc::new(VecModel::from_iter(
        activity.result_details.into_iter().map(SharedString::from),
    ))));
    window.set_status_message(SharedString::default());
    window.set_page(16);
}

fn retry(weak: &slint::Weak<AppWindow>) {
    let Some(window) = weak.upgrade() else { return };
    if !window.get_activity_result_can_retry() {
        return;
    }
    let connection_id = window.get_activity_result_connection_id();
    if connection_id.is_empty() {
        return;
    }
    let direction = window.get_activity_result_retry_direction();
    window.invoke_request_run_connection(connection_id.clone());
    if window.get_page() != 10 || window.get_run_connection_id() != connection_id {
        return;
    }
    window.set_run_direction(direction);
    window.invoke_begin_preflight();
}

fn retryable(state: QueueState) -> bool {
    matches!(state, QueueState::Failed | QueueState::Cancelled)
}

fn direction_index(direction: Direction) -> i32 {
    match direction {
        Direction::Upload => 0,
        Direction::Download => 1,
        Direction::BothWays => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::{direction_index, retryable};
    use crate::{planning::Direction, queue::QueueState};

    #[test]
    fn only_incomplete_activity_results_can_retry() {
        assert!(retryable(QueueState::Failed));
        assert!(retryable(QueueState::Cancelled));
        assert!(!retryable(QueueState::Completed));
        assert!(!retryable(QueueState::Queued));
        assert!(!retryable(QueueState::Running));
    }

    #[test]
    fn retry_preserves_the_original_direction_for_preflight() {
        assert_eq!(direction_index(Direction::Upload), 0);
        assert_eq!(direction_index(Direction::Download), 1);
        assert_eq!(direction_index(Direction::BothWays), 2);
    }
}
