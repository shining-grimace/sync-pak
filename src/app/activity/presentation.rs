mod results;

use self::results::result_details;

use crate::{
    configuration::SyncMode,
    operations::queue::{QueueEntry, QueueState},
    preflight::planning::Direction,
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
        let result_summary = entry.result.as_ref().map_or_else(
            String::new,
            crate::operations::execution::ExecutionResult::summary,
        );
        let result_details = entry
            .result
            .as_ref()
            .map_or_else(Vec::new, |result| result_details(result, &entry.snapshot));
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
    if entry.state == QueueState::Running {
        entry.progress.as_ref().map_or_else(
            String::new,
            crate::operations::operation_progress::OperationProgress::summary,
        )
    } else {
        String::new()
    }
}

fn detail(snapshot: &crate::operations::queue::snapshot::ActivitySnapshot) -> String {
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
        QueueState::ReviewRequired => "Review required",
        QueueState::Cancelled => "Cancelled",
    }
}

#[cfg(test)]
mod tests;
