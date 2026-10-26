use crate::{
    inventory::RelativePath,
    operations::archive::{
        download::download_and_create_archive,
        execution::ArchiveSource,
        history::ArchiveHistory,
        naming::archive_filename,
        store::{ArchiveRetention, ArchiveStoreError, create_upload_and_prune_archive},
    },
    operations::cancellation::CancellationToken,
    operations::request::RunRequest,
    operations::retry::RetrySleeper,
    operations::transfer::local_remote::LocalRemoteTransfer,
    providers::s3::transport::S3Transport,
};

/// Local staging and retention history for archive execution.
pub(crate) struct ArchiveStorage<'a> {
    pub history: &'a ArchiveHistory,
    pub staging_directory: &'a std::path::Path,
}

pub(super) async fn create_upload_archive(
    request: &RunRequest,
    source: ArchiveSource<'_>,
    transfer: &LocalRemoteTransfer<'_, S3Transport, impl RetrySleeper>,
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
        source,
        ArchiveRetention {
            connection_id,
            existing: &existing,
            keep_last,
        },
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
pub(super) async fn create_download_archive(
    source: ArchiveSource<'_>,
    transfer: &LocalRemoteTransfer<'_, S3Transport, impl RetrySleeper>,
    cancellation: &CancellationToken,
    jitter_seed: u64,
) -> bool {
    let Ok(filename) = archive_filename(source.timestamp, source.connection_name) else {
        return false;
    };
    let Ok(path) = RelativePath::new(filename) else {
        return false;
    };
    let destination = source
        .staging_directory
        .join(format!(".syncpak-archive-{}.tmp", uuid::Uuid::new_v4()));
    let result = download_and_create_archive(
        transfer,
        source.inventory,
        source.staging_directory,
        &destination,
        cancellation,
        jitter_seed,
    )
    .await;
    if result.is_err() {
        let _ = std::fs::remove_file(destination);
        return false;
    }
    let copied = source.source_root.copy_from(&path, &destination).is_ok();
    let _ = std::fs::remove_file(destination);
    copied
}
