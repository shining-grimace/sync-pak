use std::collections::BTreeMap;

use crate::inventory::InventoryEntryKind;

use super::CacheNamespace;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteIdentity {
    pub byte_size: u64,
    pub modified_unix_seconds: Option<i64>,
    pub entity_tag: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalFingerprint {
    pub kind: InventoryEntryKind,
    pub byte_size: u64,
    pub modified_unix_seconds: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RemoteObservation {
    pub namespace: CacheNamespace,
    pub bucket: String,
    pub key: String,
    pub identity: RemoteIdentity,
    pub source_modified_unix_seconds: Option<i64>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Baseline {
    pub namespace: CacheNamespace,
    pub path: String,
    pub local: LocalFingerprint,
    pub remote: RemoteIdentity,
    pub effective_source_unix_seconds: Option<i64>,
}

#[derive(Clone, Debug, Default)]
pub struct CacheSnapshot {
    observations: BTreeMap<(String, String), RemoteObservation>,
    baselines: BTreeMap<String, Baseline>,
}

impl CacheSnapshot {
    pub fn observation(&self, bucket: &str, key: &str) -> Option<&RemoteObservation> {
        self.observations.get(&(bucket.to_owned(), key.to_owned()))
    }

    pub fn baseline(&self, path: &str) -> Option<&Baseline> {
        self.baselines.get(path)
    }

    pub(crate) fn insert_observation(&mut self, observation: RemoteObservation) {
        self.observations.insert(
            (observation.bucket.clone(), observation.key.clone()),
            observation,
        );
    }

    pub(crate) fn insert_baseline(&mut self, baseline: Baseline) {
        self.baselines.insert(baseline.path.clone(), baseline);
    }
}

impl RemoteIdentity {
    pub fn from_metadata(metadata: &crate::providers::capabilities::ObjectMetadata) -> Self {
        Self {
            byte_size: metadata.byte_size,
            modified_unix_seconds: metadata.modified_unix_seconds,
            entity_tag: metadata.entity_tag.clone(),
        }
    }

    pub fn is_cacheable(&self) -> bool {
        self.modified_unix_seconds.is_some() && self.entity_tag.is_some()
    }
}
