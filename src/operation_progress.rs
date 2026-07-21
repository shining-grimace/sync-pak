/// The current user-visible phase of an active operation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OperationPhase {
    Preparing,
    Copying,
    Finalizing,
    Deleting,
    PruningArchives,
    CleaningUp,
}

impl OperationPhase {
    pub fn label(self) -> &'static str {
        match self {
            Self::Preparing => "Preparing",
            Self::Copying => "Copying",
            Self::Finalizing => "Finalizing",
            Self::Deleting => "Deleting",
            Self::PruningArchives => "Pruning old archives",
            Self::CleaningUp => "Cleaning up",
        }
    }
}

/// A non-secret snapshot suitable for progress modals, snackbars, and Activity.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct OperationProgress {
    pub phase: OperationPhase,
    pub completed_items: usize,
    pub total_items: usize,
    pub transferred_bytes: u64,
    pub total_bytes: u64,
    pub current_path: Option<String>,
    pub retry: Option<RetryStatus>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct RetryStatus {
    pub next_attempt: u8,
    pub max_attempts: u8,
    pub delay_millis: u64,
}

impl RetryStatus {
    pub fn summary(self) -> String {
        format!(
            "Trying again in {} ms (attempt {} of {})",
            self.delay_millis, self.next_attempt, self.max_attempts
        )
    }
}

impl Default for OperationProgress {
    fn default() -> Self {
        Self {
            phase: OperationPhase::Preparing,
            completed_items: 0,
            total_items: 0,
            transferred_bytes: 0,
            total_bytes: 0,
            current_path: None,
            retry: None,
        }
    }
}

impl OperationProgress {
    pub fn summary(&self) -> String {
        if let Some(retry) = self.retry {
            return retry.summary();
        }
        if self.total_items == 0 {
            return self.phase.label().into();
        }
        format!(
            "{} · {} of {} items · {} of {} bytes",
            self.phase.label(),
            self.completed_items,
            self.total_items,
            self.transferred_bytes,
            self.total_bytes
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{OperationPhase, OperationProgress, RetryStatus};

    #[test]
    fn progress_has_a_useful_initial_and_transfer_summary() {
        assert_eq!(OperationProgress::default().summary(), "Preparing");
        assert_eq!(
            OperationProgress {
                phase: OperationPhase::Copying,
                completed_items: 2,
                total_items: 5,
                transferred_bytes: 20,
                total_bytes: 100,
                current_path: Some("photo.jpg".into()),
                retry: None,
            }
            .summary(),
            "Copying · 2 of 5 items · 20 of 100 bytes"
        );
        assert_eq!(
            OperationProgress {
                retry: Some(RetryStatus {
                    next_attempt: 2,
                    max_attempts: 4,
                    delay_millis: 250
                }),
                ..Default::default()
            }
            .summary(),
            "Trying again in 250 ms (attempt 2 of 4)"
        );
    }
}
