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
    pub can_cancel: bool,
    pub can_remove: bool,
}

impl ActivityPresentation {
    pub fn from_entry(entry: &QueueEntry) -> Self {
        let result_summary = entry.result.as_ref().map_or_else(String::new, summary);
        Self {
            operation_id: entry.operation_id.to_string(),
            title: entry.snapshot.connection_name.clone(),
            detail: detail(&entry.snapshot),
            status: status(entry.state),
            progress_summary: entry.progress.as_ref().map_or_else(
                String::new,
                crate::operation_progress::OperationProgress::summary,
            ),
            result_summary,
            can_cancel: entry.state == QueueState::Running,
            can_remove: entry.state == QueueState::Queued,
        }
    }
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
        ExecutionState::Failed => format!(
            "{} completed · {} incomplete · {} not started",
            result.completed.len(),
            result.incomplete.len(),
            result.not_started.len()
        ),
        ExecutionState::Cancelled => format!(
            "{} completed · {} incomplete · {} not started",
            result.completed.len(),
            result.incomplete.len(),
            result.not_started.len()
        ),
        ExecutionState::Preparing | ExecutionState::Copying | ExecutionState::Finalizing => {
            String::new()
        }
    }
}

#[cfg(test)]
#[path = "activity_presentation_tests.rs"]
mod tests;
