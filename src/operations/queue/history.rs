//! Durable, non-executable Activity records. An interrupted run is never replayed.
use super::{OperationQueue, QueueEntry, QueueState, snapshot::ActivitySnapshot};
use crate::{operations::execution::ExecutionResult, preflight::planning::OperationPlan};
use serde::{Deserialize, Serialize};
use std::{io, path::PathBuf};

#[derive(Serialize, Deserialize)]
struct Record {
    id: String,
    plan: OperationPlan,
    snapshot: ActivitySnapshot,
    state: QueueState,
    result: Option<ExecutionResult>,
}
impl OperationQueue {
    pub fn restore(path: PathBuf) -> io::Result<Self> {
        let records: Vec<Record> = match std::fs::read(&path) {
            Ok(bytes) => serde_json::from_slice(&bytes).map_err(io::Error::other)?,
            Err(e) if e.kind() == io::ErrorKind::NotFound => Vec::new(),
            Err(e) => return Err(e),
        };
        let mut queue = Self {
            history_path: Some(path),
            ..Self::default()
        };
        for mut record in records {
            if matches!(record.state, QueueState::Running | QueueState::Queued) {
                record.state = QueueState::Failed;
                record.result = Some(ExecutionResult::failed_before_start_with_message(
                    "The app stopped before this operation's outcome was recorded. Some files may have changed. Check both folders before running again.",
                ));
            }
            queue.entries.push_back(QueueEntry {
                operation_id: uuid::Uuid::parse_str(&record.id).map_err(io::Error::other)?,
                plan: record.plan,
                snapshot: record.snapshot,
                state: record.state,
                result: record.result,
                confirmed_operation: None,
                automatic_request: None,
                configuration_path: None,
                progress: None,
                cancellation_requested: false,
            });
        }
        queue.persist();
        Ok(queue)
    }

    pub(super) fn persist(&mut self) {
        let Some(path) = &self.history_path else {
            return;
        };
        let records: Vec<_> = self
            .entries
            .iter()
            .map(|e| Record {
                id: e.operation_id.to_string(),
                plan: e.plan.clone(),
                snapshot: e.snapshot.clone(),
                state: e.state,
                result: e.result.clone(),
            })
            .collect();
        let result = serde_json::to_vec(&records)
            .map_err(io::Error::other)
            .and_then(|bytes| crate::platform::atomic_write::atomic_write(path, &bytes));
        self.history_error = result.err().map(|_| "Activity history could not be saved. Results may be unavailable after closing the app.".into());
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn interrupted_activity_is_reported_without_replaying_work() {
        let path = std::env::temp_dir().join(format!(
            "syncpak-history-{}/activity.json",
            uuid::Uuid::new_v4()
        ));
        let config = crate::configuration::lists::tests::config();
        let connection = &config.connections[0];
        let mut queue = OperationQueue::restore(path.clone()).unwrap();
        queue.push(
            OperationPlan::new(
                connection.id.as_str(),
                connection.mode,
                crate::preflight::planning::Direction::Upload,
            ),
            ActivitySnapshot::from_connection(
                connection,
                "Cloud",
                crate::preflight::planning::Direction::Upload,
            ),
        );
        let restored = OperationQueue::restore(path.clone()).unwrap();
        let entry = restored.activity().next().unwrap();
        assert_eq!(entry.state, QueueState::Failed);
        assert!(
            entry
                .result
                .as_ref()
                .unwrap()
                .failure_message
                .as_ref()
                .unwrap()
                .contains("Some files may have changed")
        );
        assert!(entry.confirmed_operation.is_none());
        std::fs::remove_dir_all(path.parent().unwrap()).unwrap();
    }
}
