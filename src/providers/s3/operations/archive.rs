mod storage;
mod timestamp;
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
    let action =
        preflight
            .plan()
            .actions()
            .first()
            .cloned()
            .unwrap_or(PlannedAction::CreateArchive {
                from: crate::preflight::planning::Endpoint::Source,
                to: crate::preflight::planning::Endpoint::Destination,
            });
    let mut progress = ExecutionProgress::new([action.clone()]);
    if cancellation.is_cancelled() {
        return finish(
            &mut progress,
            observer,
            ExecutionState::Cancelled,
            0,
            1,
            None,
        );
    }
    observer.on_progress(&TransferProgress {
        state: ExecutionState::Copying,
        completed_actions: 0,
        total_actions: 1,
        current_action: Some(action),
    });
    let root = LocalTransferRoot::from_config(&request.connection.local_path);
    let timestamp = timestamp::utc_timestamp();
    let source = ArchiveSource {
        source_root: &root,
        inventory: preflight.source(),
        staging_directory: storage.staging_directory,
        timestamp: &timestamp,
        connection_name: &request.connection.name,
    };
    let success = match request.direction {
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
        Direction::BothWays => false,
    };
    if success {
        progress.complete_current();
        finish(
            &mut progress,
            observer,
            ExecutionState::Completed,
            1,
            1,
            None,
        )
    } else if cancellation.is_cancelled() {
        finish(
            &mut progress,
            observer,
            ExecutionState::Cancelled,
            0,
            1,
            None,
        )
    } else {
        let mut result = finish(&mut progress, observer, ExecutionState::Failed, 0, 1, None);
        result.failure_message = Some(
            "Archive creation or storage failed. A recoverable staging file was kept when possible."
                .into(),
        );
        result
    }
}

fn finish(
    progress: &mut ExecutionProgress,
    observer: &dyn TransferProgressObserver,
    state: ExecutionState,
    completed: usize,
    total: usize,
    action: Option<PlannedAction>,
) -> ExecutionResult {
    observer.on_progress(&TransferProgress {
        state,
        completed_actions: completed,
        total_actions: total,
        current_action: action,
    });
    match state {
        ExecutionState::Completed => {
            std::mem::replace(progress, ExecutionProgress::new([])).finish()
        }
        ExecutionState::Cancelled => {
            std::mem::replace(progress, ExecutionProgress::new([])).cancel()
        }
        ExecutionState::Failed => std::mem::replace(progress, ExecutionProgress::new([])).fail(),
        _ => unreachable!(),
    }
}
