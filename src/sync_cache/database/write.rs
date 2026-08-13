use rusqlite::{Transaction, params};

use crate::sync_cache::{Baseline, RemoteObservation};

use super::codec::{encode_kind, unix_now};

pub fn write_observation(
    transaction: &Transaction<'_>,
    observation: &RemoteObservation,
) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT OR REPLACE INTO remote_observations VALUES
         (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
        params![
            observation.namespace.as_str(),
            observation.bucket,
            observation.key,
            observation.identity.byte_size.to_string(),
            observation.identity.modified_unix_seconds,
            observation.identity.entity_tag,
            observation.source_modified_unix_seconds,
            unix_now(),
        ],
    )?;
    Ok(())
}

pub fn write_baseline(transaction: &Transaction<'_>, baseline: &Baseline) -> rusqlite::Result<()> {
    transaction.execute(
        "INSERT OR REPLACE INTO baselines VALUES
         (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)",
        params![
            baseline.namespace.as_str(),
            baseline.path,
            encode_kind(&baseline.local.kind),
            baseline.local.byte_size.to_string(),
            baseline.local.modified_unix_seconds,
            baseline.remote.byte_size.to_string(),
            baseline.remote.modified_unix_seconds,
            baseline.remote.entity_tag,
            baseline.effective_source_unix_seconds,
            unix_now(),
        ],
    )?;
    Ok(())
}
