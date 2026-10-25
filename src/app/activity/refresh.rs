use crate::{
    ActivityRow, AppWindow,
    app::activity::presentation::ActivityPresentation,
    operations::{execution::OperationExecutor, queue::background::BackgroundQueue},
};
use slint::{Model, SharedString};

fn progress_percent(progress: &crate::operations::operation_progress::OperationProgress) -> i32 {
    progress
        .transferred_bytes
        .saturating_mul(100)
        .checked_div(progress.total_bytes)
        .map(|p| p.min(100) as i32)
        .or_else(|| {
            progress
                .completed_items
                .saturating_mul(100)
                .checked_div(progress.total_items)
                .map(|p| p.min(100) as i32)
        })
        .unwrap_or(0)
}

pub(super) fn refresh<E: OperationExecutor + Send + Sync + 'static>(
    weak: &slint::Weak<AppWindow>,
    queue: &BackgroundQueue<E>,
) {
    let Some(window) = weak.upgrade() else { return };
    if let Some(message) = queue.take_history_error()
        && window.get_status_message().as_str() != message
    {
        window.set_status_message(message.into());
    }
    let activity = queue.activity();
    let busy = activity.iter().find(|entry| {
        matches!(
            entry.state,
            crate::operations::queue::QueueState::Queued
                | crate::operations::queue::QueueState::Running
        )
    });
    window.set_operation_busy(busy.is_some());
    window.set_busy_connection_id(
        busy.map_or_else(Default::default, |e| e.plan.connection_id.clone().into()),
    );
    window.set_busy_provider_id(
        busy.and_then(|e| {
            e.automatic_request
                .as_ref()
                .or_else(|| e.confirmed_operation.as_ref().map(|c| c.request()))
        })
        .map_or_else(Default::default, |r| r.provider.id.as_str().into()),
    );
    let connections = window.get_connections();
    for (index, mut row) in connections.iter().enumerate() {
        let running = busy.is_some_and(|e| e.plan.connection_id == row.id.as_str());
        let result_id: SharedString = activity
            .iter()
            .find(|e| e.plan.connection_id == row.id.as_str() && e.result.is_some())
            .map_or_else(Default::default, |e| e.operation_id.to_string().into());
        if row.running != running || row.result_id != result_id {
            if row.result_id != result_id
                && let Some(entry) = activity
                    .iter()
                    .find(|e| e.operation_id.to_string() == result_id.as_str())
                && let Some(path) = &entry.configuration_path
                && let Ok(config) = crate::configuration::ConfigStore::at(path.clone()).load()
                && let Some(connection) = config
                    .connections
                    .iter()
                    .find(|c| c.id.as_str() == row.id.as_str())
            {
                row.last_run = crate::app::connections::last_run::label(connection.last_run).into();
            }
            row.running = running;
            row.result_id = result_id;
            connections.set_row_data(index, row);
        }
    }
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
    window.set_activity_rows(super::rows::update(
        window.get_activity_rows(),
        rows.collect(),
    ));
}
