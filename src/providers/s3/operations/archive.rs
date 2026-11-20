mod storage;
mod timestamp;
use std::future::Future;
pub(super) use storage::ArchiveStorage;
use storage::{create_download_archive, create_upload_archive};

use crate::{
    operations::archive::execution::ArchiveSource,
    operations::cancellation::CancellationToken,
    operations::execution::{ExecutionProgress, ExecutionResult, ExecutionState},
    operations::request::RunRequest,
    operations::retry::RetrySleeper,
    operations::transfer::local_remote::LocalRemoteTransfer,
    operations::transfer::paths::LocalTransferRoot,
    operations::transfer::progress::{TransferProgress, TransferProgressObserver},
    preflight::Preflight,
    preflight::planning::{Direction, PlannedAction},
    providers::s3::transport::S3Transport,
};

/// Executes the single reviewed archive action in the selected direction.
pub(crate) async fn execute(
    request: &RunRequest,
    preflight: &Preflight,
    transfer: &LocalRemoteTransfer<'_, S3Transport, impl RetrySleeper>,
    cancellation: &CancellationToken,
    storage: ArchiveStorage<'_>,
    observer: &dyn TransferProgressObserver,
    jitter_seed: u64,
) -> ExecutionResult {
    let Some(action) = preflight.plan().actions().first().cloned() else {
        return ExecutionResult::failed_before_start_with_message(
            "No archive action was reviewed.",
        );
    };
    let root = LocalTransferRoot::from_config(&request.connection.local_path);
    let timestamp = timestamp::utc_timestamp();
    let inventory = preflight.archive_source();
    let source = ArchiveSource {
        source_root: &root,
        inventory: &inventory,
        staging_directory: storage.staging_directory,
        timestamp: &timestamp,
        connection_name: &request.connection.name,
    };
    let store = async {
        match request.direction {
            Direction::Upload => {
                // The archive helpers each contain a deep generic async transfer stack.
                // Keep that stack behind a heap boundary so this dispatcher future stays small.
                Box::pin(create_upload_archive(
                    request,
                    source,
                    transfer,
                    cancellation,
                    storage.history,
                    jitter_seed,
                ))
                .await
            }
            Direction::Download => {
                Box::pin(create_download_archive(
                    source,
                    transfer,
                    cancellation,
                    jitter_seed,
                ))
                .await
            }
            Direction::BothWays => unreachable!("archive preflight has exactly one direction"),
        }
    };
    execute_action(action, cancellation, observer, store).await
}

async fn execute_action(
    action: PlannedAction,
    cancellation: &CancellationToken,
    observer: &dyn TransferProgressObserver,
    store: impl Future<Output = bool>,
) -> ExecutionResult {
    let mut progress = ExecutionProgress::new([action.clone()]);
    let result = if cancellation.is_cancelled() {
        progress.cancel()
    } else {
        progress.start_next();
        observer.on_progress(&TransferProgress {
            state: ExecutionState::Copying,
            completed_actions: 0,
            total_actions: 1,
            current_action: Some(action),
        });
        if store.await {
            progress.complete_current();
            progress.finish()
        } else if cancellation.is_cancelled() {
            progress.cancel()
        } else {
            let mut result = progress.fail();
            result.failure_message = Some(
                "Archive creation or storage failed. A recoverable staging file was kept when possible."
                    .into(),
            );
            result
        }
    };
    observer.on_progress(&TransferProgress {
        state: result.state,
        completed_actions: result.completed.len(),
        total_actions: 1,
        current_action: None,
    });
    result
}

#[cfg(test)]
mod tests;
