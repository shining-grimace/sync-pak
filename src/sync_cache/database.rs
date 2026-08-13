use std::{path::Path, time::Duration};

use rusqlite::{Connection, params};

mod codec;
mod write;

use codec::{decode_kind, parse_size, unix_now};
use write::{write_baseline, write_observation};

use super::{
    Baseline, CacheNamespace, LocalFingerprint, RemoteIdentity, RemoteObservation,
    model::CacheSnapshot,
};

const RETENTION_SECONDS: i64 = 180 * 24 * 60 * 60;

pub struct CacheDatabase(Connection);

impl CacheDatabase {
    pub fn prepare(path: &Path) -> rusqlite::Result<()> {
        match Self::prepare_once(path) {
            Ok(()) => Ok(()),
            Err(first_error) => {
                eprintln!("SyncPak metadata cache was invalid and will be rebuilt.");
                if path.exists() {
                    preserve_damaged_database(path).map_err(|_| first_error)?;
                }
                Self::prepare_once(path)
            }
        }
    }

    fn prepare_once(path: &Path) -> rusqlite::Result<()> {
        let database = Self::open(path)?;
        let schema_version: i64 = database
            .0
            .query_row("PRAGMA user_version", [], |row| row.get(0))?;
        if !matches!(schema_version, 0 | 1) {
            return Err(rusqlite::Error::InvalidQuery);
        }
        database.0.execute_batch(SCHEMA)?;
        let check: String = database
            .0
            .query_row("PRAGMA quick_check", [], |row| row.get(0))?;
        if check != "ok" {
            return Err(rusqlite::Error::InvalidQuery);
        }
        let cutoff = unix_now().saturating_sub(RETENTION_SECONDS);
        database.0.execute(
            "DELETE FROM remote_observations WHERE last_seen < ?1",
            [cutoff],
        )?;
        database
            .0
            .execute("DELETE FROM baselines WHERE last_seen < ?1", [cutoff])?;
        Ok(())
    }

    pub fn open(path: &Path) -> rusqlite::Result<Self> {
        let connection = Connection::open(path)?;
        connection.busy_timeout(Duration::from_secs(2))?;
        connection.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")?;
        Ok(Self(connection))
    }

    pub fn snapshot(&self, namespace: &CacheNamespace) -> rusqlite::Result<CacheSnapshot> {
        let mut snapshot = CacheSnapshot::default();
        self.load_observations(namespace, &mut snapshot)?;
        self.load_baselines(namespace, &mut snapshot)?;
        Ok(snapshot)
    }

    pub fn record_observations(
        &mut self,
        observations: &[RemoteObservation],
    ) -> rusqlite::Result<()> {
        let transaction = self.0.transaction()?;
        for observation in observations {
            write_observation(&transaction, observation)?;
        }
        transaction.commit()
    }

    pub fn record_transfer(
        &mut self,
        observation: &RemoteObservation,
        baseline: &Baseline,
    ) -> rusqlite::Result<()> {
        let transaction = self.0.transaction()?;
        write_observation(&transaction, observation)?;
        write_baseline(&transaction, baseline)?;
        transaction.commit()
    }

    pub fn invalidate(
        &mut self,
        namespace: &CacheNamespace,
        bucket: &str,
        key: &str,
        path: &str,
    ) -> rusqlite::Result<()> {
        let transaction = self.0.transaction()?;
        transaction.execute(
            "DELETE FROM remote_observations
             WHERE namespace = ?1 AND bucket = ?2 AND object_key = ?3",
            params![namespace.as_str(), bucket, key],
        )?;
        transaction.execute(
            "DELETE FROM baselines WHERE namespace = ?1 AND path = ?2",
            params![namespace.as_str(), path],
        )?;
        transaction.commit()
    }

    fn load_observations(
        &self,
        namespace: &CacheNamespace,
        snapshot: &mut CacheSnapshot,
    ) -> rusqlite::Result<()> {
        let mut statement = self.0.prepare(
            "SELECT bucket, object_key, byte_size, modified, etag, source_modified
             FROM remote_observations WHERE namespace = ?1",
        )?;
        let rows = statement.query_map([namespace.as_str()], |row| {
            Ok(RemoteObservation {
                namespace: namespace.clone(),
                bucket: row.get(0)?,
                key: row.get(1)?,
                identity: RemoteIdentity {
                    byte_size: parse_size(row.get(2)?)?,
                    modified_unix_seconds: row.get(3)?,
                    entity_tag: row.get(4)?,
                },
                source_modified_unix_seconds: row.get(5)?,
            })
        })?;
        for row in rows {
            snapshot.insert_observation(row?);
        }
        Ok(())
    }

    fn load_baselines(
        &self,
        namespace: &CacheNamespace,
        snapshot: &mut CacheSnapshot,
    ) -> rusqlite::Result<()> {
        let mut statement = self.0.prepare(
            "SELECT path, local_kind, local_size, local_modified, remote_size,
                    remote_modified, remote_etag, effective_source
             FROM baselines WHERE namespace = ?1",
        )?;
        let rows = statement.query_map([namespace.as_str()], |row| {
            Ok(Baseline {
                namespace: namespace.clone(),
                path: row.get(0)?,
                local: LocalFingerprint {
                    kind: decode_kind(row.get(1)?)?,
                    byte_size: parse_size(row.get(2)?)?,
                    modified_unix_seconds: row.get(3)?,
                },
                remote: RemoteIdentity {
                    byte_size: parse_size(row.get(4)?)?,
                    modified_unix_seconds: row.get(5)?,
                    entity_tag: row.get(6)?,
                },
                effective_source_unix_seconds: row.get(7)?,
            })
        })?;
        for row in rows {
            snapshot.insert_baseline(row?);
        }
        Ok(())
    }
}

fn preserve_damaged_database(path: &Path) -> std::io::Result<()> {
    let suffix = uuid::Uuid::new_v4();
    std::fs::rename(path, path.with_extension(format!("corrupt-{suffix}")))?;
    for sidecar_suffix in ["-wal", "-shm"] {
        let sidecar = std::path::PathBuf::from(format!("{}{sidecar_suffix}", path.display()));
        if sidecar.exists() {
            let damaged = std::path::PathBuf::from(format!(
                "{}.corrupt-{suffix}{sidecar_suffix}",
                path.with_extension("").display()
            ));
            let _ = std::fs::rename(sidecar, damaged);
        }
    }
    Ok(())
}

const SCHEMA: &str = include_str!("schema.sql");
