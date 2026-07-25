use crate::{
    archive_download::download_and_create_archive,
    archive_history::ArchiveHistory,
    archive_naming::archive_filename,
    archive_store::{ArchiveStoreError, create_upload_and_prune_archive},
    cancellation::CancellationToken,
    execution::{ExecutionProgress, ExecutionResult, ExecutionState},
    inventory::RelativePath,
    local_remote_transfer::LocalRemoteTransfer,
    planning::{Direction, PlannedAction},
    preflight::Preflight,
    retry::RetrySleeper,
    run_request::RunRequest,
    s3_transport::S3Transport,
    transfer_paths::LocalTransferRoot,
    transfer_progress::{TransferProgress, TransferProgressObserver},
};
use std::time::{SystemTime, UNIX_EPOCH};

/// Executes the single reviewed archive action in the selected direction.
pub(crate) async fn execute(
    request: &RunRequest,
    preflight: &Preflight,
    transfer: &LocalRemoteTransfer<'_, S3Transport, impl RetrySleeper>,
    cancellation: &CancellationToken,
    history: &ArchiveHistory,
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
                from: crate::planning::Endpoint::Source,
                to: crate::planning::Endpoint::Destination,
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
    let root = LocalTransferRoot::new(&request.connection.local_path);
    let timestamp = utc_timestamp();
    let success = match request.direction {
        Direction::Upload => {
            create_upload_archive(
                request,
                preflight,
                transfer,
                &root,
                &timestamp,
                cancellation,
                history,
                jitter_seed,
            )
            .await
        }
        Direction::Download => {
            create_download_archive(
                preflight,
                transfer,
                &root,
                &timestamp,
                &request.connection.name,
                cancellation,
                jitter_seed,
            )
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

async fn create_upload_archive(
    request: &RunRequest,
    preflight: &Preflight,
    transfer: &LocalRemoteTransfer<'_, S3Transport, impl RetrySleeper>,
    root: &LocalTransferRoot,
    timestamp: &str,
    cancellation: &CancellationToken,
    history: &ArchiveHistory,
    jitter_seed: u64,
) -> bool {
    let connection_id = &request.connection.id;
    let Ok(existing) = history.load(connection_id) else {
        return false;
    };
    let Some(keep_last) = request.connection.keep_last_archives else {
        return false;
    };
    let stored = create_upload_and_prune_archive(
        root,
        preflight.source(),
        root.as_path(),
        timestamp,
        connection_id,
        &request.connection.name,
        &existing,
        keep_last,
        transfer,
        transfer,
        cancellation,
        jitter_seed,
    )
    .await;
    let (archive, pruned, completed) = match stored {
        Ok(stored) => (stored.archive, stored.pruned, true),
        Err(ArchiveStoreError::Prune { archive, .. }) => (archive, Vec::new(), false),
        Err(ArchiveStoreError::Store(_)) => return false,
    };
    let mut retained = existing;
    retained.push(archive);
    retained.retain(|record| {
        !pruned
            .iter()
            .any(|pruned| pruned.location == record.location)
    });
    history.save(connection_id, &retained).is_ok() && completed
}
async fn create_download_archive(
    preflight: &Preflight,
    transfer: &LocalRemoteTransfer<'_, S3Transport, impl RetrySleeper>,
    root: &LocalTransferRoot,
    timestamp: &str,
    connection_name: &str,
    cancellation: &CancellationToken,
    jitter_seed: u64,
) -> bool {
    let Ok(filename) = archive_filename(timestamp, connection_name) else {
        return false;
    };
    let Ok(path) = RelativePath::new(filename) else {
        return false;
    };
    download_and_create_archive(
        transfer,
        preflight.source(),
        root.as_path(),
        &root.resolve(&path),
        cancellation,
        jitter_seed,
    )
    .await
    .is_ok()
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

fn utc_timestamp() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |duration| duration.as_secs());
    let days = (seconds / 86_400) as i64;
    let seconds_of_day = seconds % 86_400;
    let (year, month, day) = civil_date(days);
    format!(
        "{year:04}{month:02}{day:02}-{:02}{:02}{:02}Z",
        seconds_of_day / 3_600,
        seconds_of_day / 60 % 60,
        seconds_of_day % 60,
    )
}

fn civil_date(days_since_epoch: i64) -> (i64, i64, i64) {
    let days = days_since_epoch + 719_468;
    let era = if days >= 0 { days } else { days - 146_096 } / 146_097;
    let day_of_era = days - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = month_index + if month_index < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::civil_date;

    #[test]
    fn converts_unix_epoch_days_to_a_gregorian_date() {
        assert_eq!(civil_date(0), (1970, 1, 1));
        assert_eq!(civil_date(20_000), (2024, 10, 4));
    }
}
