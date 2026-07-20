use crate::{
    execution::ExecutionState,
    queue::{QueueEntry, QueueState},
};

/// UI-ready, non-secret information for one in-memory Activity entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivityPresentation {
    pub operation_id: String,
    pub title: String,
    pub detail: String,
    pub status: &'static str,
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
            detail: format!(
                "{} → {}",
                entry.snapshot.local_endpoint, entry.snapshot.remote_endpoint
            ),
            status: status(entry.state),
            result_summary,
            can_cancel: entry.state == QueueState::Running,
            can_remove: entry.state == QueueState::Queued,
        }
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
