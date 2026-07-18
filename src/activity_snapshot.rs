use crate::{
    configuration::{ConnectionConfig, SyncMode},
    planning::Direction,
};

/// Non-secret connection details retained with an in-memory activity entry.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ActivitySnapshot {
    pub connection_name: String,
    pub mode: SyncMode,
    pub direction: Direction,
    pub local_endpoint: String,
    pub remote_endpoint: String,
}

impl ActivitySnapshot {
    pub fn from_connection(
        connection: &ConnectionConfig,
        provider_name: impl Into<String>,
        direction: Direction,
    ) -> Self {
        let provider_name = provider_name.into();
        Self {
            connection_name: connection.name.clone(),
            mode: connection.mode,
            direction,
            local_endpoint: connection.local_path.clone(),
            remote_endpoint: remote_endpoint(
                &provider_name,
                &connection.bucket,
                &connection.remote_path,
            ),
        }
    }
}

fn remote_endpoint(provider_name: &str, bucket: &str, remote_path: &str) -> String {
    if remote_path.is_empty() {
        format!("{provider_name} / {bucket}")
    } else {
        format!("{provider_name} / {bucket} / {remote_path}")
    }
}

#[cfg(test)]
mod tests {
    use crate::{
        configuration::{ConnectionConfig, ConnectionId, ProviderId, SyncMode},
        planning::Direction,
    };

    use super::ActivitySnapshot;

    #[test]
    fn snapshot_copies_non_secret_connection_details() {
        let connection = ConnectionConfig {
            id: ConnectionId::new(),
            name: "Photos".into(),
            provider_id: ProviderId::new(),
            bucket: "backups".into(),
            remote_path: "phone".into(),
            local_path: "/pictures".into(),
            mode: SyncMode::Mirror,
            keep_last_archives: None,
        };

        let snapshot = ActivitySnapshot::from_connection(&connection, "R2", Direction::Upload);

        assert_eq!(snapshot.connection_name, "Photos");
        assert_eq!(snapshot.local_endpoint, "/pictures");
        assert_eq!(snapshot.remote_endpoint, "R2 / backups / phone");
        assert_eq!(snapshot.mode, SyncMode::Mirror);
    }
}
