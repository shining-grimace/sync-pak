mod database;
mod identity;
mod model;

pub use identity::CacheNamespace;
pub(crate) use model::{Baseline, RemoteObservation};
pub use model::{CacheSnapshot, LocalFingerprint, RemoteIdentity};

use std::path::{Path, PathBuf};

use database::CacheDatabase;
#[derive(Clone, Debug)]
pub struct SyncCache {
    path: PathBuf,
    installation_id: String,
}

impl SyncCache {
    pub fn for_configuration(configuration_path: &Path) -> Option<Self> {
        let directory = configuration_path.parent()?;
        std::fs::create_dir_all(directory).ok()?;
        let installation_id = identity::installation_id(directory)?;
        let path = directory.join("sync-metadata.sqlite3");
        if CacheDatabase::prepare(&path).is_err() {
            eprintln!("SyncPak metadata cache is unavailable; continuing without it.");
            return None;
        }
        Some(Self {
            path,
            installation_id,
        })
    }

    pub fn namespace(&self, request: &crate::operations::request::RunRequest) -> CacheNamespace {
        CacheNamespace::new(&self.installation_id, request)
    }

    pub fn snapshot(&self, namespace: &CacheNamespace) -> CacheSnapshot {
        CacheDatabase::open(&self.path)
            .and_then(|database| database.snapshot(namespace))
            .unwrap_or_default()
    }

    pub fn record_observations(&self, observations: &[RemoteObservation]) {
        if observations.is_empty() {
            return;
        }
        if let Ok(mut database) = CacheDatabase::open(&self.path) {
            let _ = database.record_observations(observations);
        }
    }

    pub fn record_transfer(&self, observation: &RemoteObservation, baseline: &Baseline) {
        if let Ok(mut database) = CacheDatabase::open(&self.path) {
            let _ = database.record_transfer(observation, baseline);
        }
    }

    pub fn invalidate(&self, namespace: &CacheNamespace, bucket: &str, key: &str, path: &str) {
        if let Ok(mut database) = CacheDatabase::open(&self.path) {
            let _ = database.invalidate(namespace, bucket, key, path);
        }
    }
}

#[cfg(test)]
mod tests;
