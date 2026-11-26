use super::ExecutionState;
use crate::preflight::planning::PlannedAction;

impl super::ExecutionResult {
    pub fn summary(&self) -> String {
        let result = self;
        match result.state {
            ExecutionState::Completed => {
                if matches!(
                    result.completed.as_slice(),
                    [PlannedAction::CreateArchive { .. }]
                ) {
                    return "Archive created.".into();
                }
                let skipped = result
                    .completed
                    .iter()
                    .filter(|action| matches!(action, PlannedAction::SkipChanged { .. }))
                    .count();
                let completed = result.completed.len() - skipped;
                let items = if completed == 1 { "item" } else { "items" };
                if result.completed.is_empty() {
                    "No changes were needed.".into()
                } else if skipped > 0 {
                    format!(
                        "{completed} {items} completed · {skipped} {} skipped",
                        if skipped == 1 { "file" } else { "files" }
                    )
                } else {
                    format!("{completed} {items} completed")
                }
            }
            ExecutionState::Failed => result
                .failure_message
                .clone()
                .unwrap_or_else(|| incomplete_summary(result, "Failed")),
            ExecutionState::ReviewRequired => result.failure_message.clone().unwrap_or_else(|| {
                "Review required. No files changed. Open SyncPak to review.".into()
            }),
            ExecutionState::Cancelled => incomplete_summary(result, "Cancelled"),
            ExecutionState::Preparing | ExecutionState::Copying | ExecutionState::Finalizing => {
                String::new()
            }
        }
    }
}

fn incomplete_summary(
    result: &crate::operations::execution::ExecutionResult,
    state: &str,
) -> String {
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
