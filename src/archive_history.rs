use std::{error::Error, fmt, fs, io, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::{
    archive_retention::ArchiveRecord, atomic_write::atomic_write, configuration::ConnectionId,
};

/// Persists only archive locations that this connection has successfully stored.
pub struct ArchiveHistory {
    directory: PathBuf,
}

impl ArchiveHistory {
    pub fn new(directory: impl Into<PathBuf>) -> Self {
        Self {
            directory: directory.into(),
        }
    }

    pub fn load(
        &self,
        connection_id: &ConnectionId,
    ) -> Result<Vec<ArchiveRecord>, ArchiveHistoryError> {
        let path = self.path(connection_id);
        let bytes = match fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(error) => return Err(ArchiveHistoryError::Io(error)),
        };
        let history: StoredHistory =
            serde_json::from_slice(&bytes).map_err(ArchiveHistoryError::Decode)?;
        Ok(history
            .records
            .into_iter()
            .map(|record| ArchiveRecord {
                connection_id: connection_id.clone(),
                location: record.location,
                created_at_utc: record.created_at_utc,
            })
            .collect())
    }

    pub fn save(
        &self,
        connection_id: &ConnectionId,
        records: &[ArchiveRecord],
    ) -> Result<(), ArchiveHistoryError> {
        let history = StoredHistory {
            records: records
                .iter()
                .map(|record| StoredRecord {
                    location: record.location.clone(),
                    created_at_utc: record.created_at_utc.clone(),
                })
                .collect(),
        };
        let bytes = serde_json::to_vec(&history).map_err(ArchiveHistoryError::Encode)?;
        atomic_write(&self.path(connection_id), &bytes).map_err(ArchiveHistoryError::Io)
    }

    fn path(&self, connection_id: &ConnectionId) -> PathBuf {
        self.directory
            .join(format!("archive-history-{}.json", connection_id.as_str()))
    }
}

#[derive(Serialize, Deserialize)]
struct StoredHistory {
    records: Vec<StoredRecord>,
}

#[derive(Serialize, Deserialize)]
struct StoredRecord {
    location: String,
    created_at_utc: String,
}

#[derive(Debug)]
pub enum ArchiveHistoryError {
    Io(io::Error),
    Decode(serde_json::Error),
    Encode(serde_json::Error),
}

impl fmt::Display for ArchiveHistoryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(_) => f.write_str("archive history could not be accessed"),
            Self::Decode(_) => f.write_str("archive history is not valid"),
            Self::Encode(_) => f.write_str("archive history could not be saved"),
        }
    }
}
impl Error for ArchiveHistoryError {}

#[cfg(test)]
mod tests {
    use crate::{archive_retention::ArchiveRecord, configuration::ConnectionId};

    use super::ArchiveHistory;

    #[test]
    fn persists_connection_owned_archive_locations() {
        let directory =
            std::env::temp_dir().join(format!("sync-pak-history-{}", uuid::Uuid::new_v4()));
        let history = ArchiveHistory::new(&directory);
        let connection_id = ConnectionId::new();
        let records = vec![ArchiveRecord {
            connection_id: connection_id.clone(),
            location: "20260726-120000Z Photos.zip".into(),
            created_at_utc: "20260726-120000Z".into(),
        }];

        history.save(&connection_id, &records).unwrap();

        assert_eq!(history.load(&connection_id).unwrap(), records);
        std::fs::remove_dir_all(directory).unwrap();
    }
}
