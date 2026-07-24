use crate::{
    configuration::SyncMode,
    execution::ExecutionState,
    planning::Direction,
    queue::{QueueEntry, QueueState},
};

/// UI-ready, non-secret information for one in-memory Activity entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivityPresentation {
    pub operation_id: String,
    pub title: String,
    pub detail: String,
    pub status: &'static str,
    pub progress_summary: String,
    pub result_summary: String,
    pub result_details: Vec<String>,
    pub can_cancel: bool,
    pub can_remove: bool,
    pub can_view_result: bool,
}

impl ActivityPresentation {
    pub fn from_entry(entry: &QueueEntry) -> Self {
        let result_summary = entry.result.as_ref().map_or_else(String::new, summary);
        let result_details = entry.result.as_ref().map_or_else(Vec::new, result_details);
        Self {
            operation_id: entry.operation_id.to_string(),
            title: entry.snapshot.connection_name.clone(),
            detail: detail(&entry.snapshot),
            status: status(entry.state),
            progress_summary: progress_summary(entry),
            result_summary,
            result_details,
            can_cancel: entry.state == QueueState::Running,
            can_remove: entry.state == QueueState::Queued,
            can_view_result: entry.result.is_some(),
        }
    }
}

fn progress_summary(entry: &QueueEntry) -> String {
    (entry.state == QueueState::Running)
        .then(|| {
            entry.progress.as_ref().map_or_else(
                String::new,
                crate::operation_progress::OperationProgress::summary,
            )
        })
        .unwrap_or_default()
}

fn detail(snapshot: &crate::activity_snapshot::ActivitySnapshot) -> String {
    format!(
        "{} · {} · {} → {}",
        mode(snapshot.mode),
        direction(snapshot.direction),
        snapshot.local_endpoint,
        snapshot.remote_endpoint
    )
}

fn mode(mode: SyncMode) -> &'static str {
    match mode {
        SyncMode::AddOnly => "Add-only",
        SyncMode::Mirror => "Mirror",
        SyncMode::Archive => "Archive",
    }
}

fn direction(direction: Direction) -> &'static str {
    match direction {
        Direction::Upload => "Upload",
        Direction::Download => "Download",
        Direction::BothWays => "Both ways",
    }
}

fn status(state: QueueState) -> &'static str {
    match state {
        QueueState::Queued => "Queued",
        QueueState::Running => "In progress",
        QueueState::Completed => "Completed",
        QueueState::Failed => "Failed",
        QueueState::Cancelled => "Cancelled",
    }
}

fn summary(result: &crate::execution::ExecutionResult) -> String {
    match result.state {
        ExecutionState::Completed => format!("{} items completed", result.completed.len()),
        ExecutionState::Failed => incomplete_summary(result, "Failed"),
        ExecutionState::Cancelled => incomplete_summary(result, "Cancelled"),
        ExecutionState::Preparing | ExecutionState::Copying | ExecutionState::Finalizing => {
            String::new()
        }
    }
}

fn incomplete_summary(result: &crate::execution::ExecutionResult, state: &str) -> String {
    if result.completed.is_empty() && result.incomplete.is_empty() && result.not_started.is_empty()
    {
        return format!("{state} before starting");
    }
    format!(
        "{} completed · {} incomplete · {} not started",
        result.completed.len(),
        result.incomplete.len(),
        result.not_started.len()
    )
}

fn result_details(result: &crate::execution::ExecutionResult) -> Vec<String> {
    let mut details = Vec::new();
    details.extend(action_details("Completed", &result.completed));
    details.extend(action_details("Incomplete", &result.incomplete));
    details.extend(action_details("Not started", &result.not_started));
    details
}

fn action_details(status: &str, actions: &[crate::planning::PlannedAction]) -> Vec<String> {
    actions
        .iter()
        .map(|action| format!("{status}: {}", action_name(action)))
        .collect()
}

fn action_name(action: &crate::planning::PlannedAction) -> String {
    use crate::planning::PlannedAction;

    match action {
        PlannedAction::Copy { path, .. }
        | PlannedAction::Overwrite { path, .. }
        | PlannedAction::Delete { path, .. }
        | PlannedAction::SkipChanged { path } => path.as_str().into(),
        PlannedAction::CreateArchive { .. } => "archive".into(),
    }
}

#[cfg(test)]
#[path = "activity_presentation_tests.rs"]
mod tests;
