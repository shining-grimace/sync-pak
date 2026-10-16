use super::ExecutionState;

impl super::ExecutionResult {
    pub fn summary(&self) -> String {
        let result = self;
        match result.state {
            ExecutionState::Completed => {
                let skipped = result
                    .completed
                    .iter()
                    .filter(|action| {
                        matches!(
                            action,
                            crate::preflight::planning::PlannedAction::SkipChanged { .. }
                        )
                    })
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
