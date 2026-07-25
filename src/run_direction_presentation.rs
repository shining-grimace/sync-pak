use crate::{
    configuration::{ConnectionConfig, SyncMode},
    planning::Direction,
};

/// Non-secret endpoint labels used by the direction-selection screen.
pub(crate) fn remote_endpoint(provider_name: &str, connection: &ConnectionConfig) -> String {
    if connection.remote_path.is_empty() {
        format!("{provider_name} · {} (bucket root)", connection.bucket)
    } else {
        format!(
            "{provider_name} · {}/{}",
            connection.bucket, connection.remote_path
        )
    }
}

pub(crate) fn archive_details(connection: &ConnectionConfig, direction: Direction) -> String {
    if connection.mode != SyncMode::Archive {
        return String::new();
    }
    match direction {
        Direction::Upload => format!(
            "A new ZIP will be stored in {}. SyncPak will keep the newest {} remote archives.",
            remote_destination(connection),
            connection.keep_last_archives.unwrap_or_default()
        ),
        Direction::Download => format!(
            "SyncPak will create a ZIP in {} from the cloud folder. Remote archive retention does not apply to this download.",
            connection.local_path
        ),
        Direction::BothWays => String::new(),
    }
}

pub(crate) fn mode_label(mode: SyncMode) -> &'static str {
    match mode {
        SyncMode::AddOnly => "Add-only",
        SyncMode::Mirror => "Mirror",
        SyncMode::Archive => "Archive",
    }
}

fn remote_destination(connection: &ConnectionConfig) -> String {
    if connection.remote_path.is_empty() {
        format!("the root of {}", connection.bucket)
    } else {
        format!("{}/{}", connection.bucket, connection.remote_path)
    }
}
