use crate::{
    configuration::{ConnectionConfig, ConnectionId, ProviderId, SyncMode},
    planning::Direction,
};

use super::archive_details;

fn archive_connection() -> ConnectionConfig {
    ConnectionConfig {
        id: ConnectionId::new(),
        name: "Photos".into(),
        provider_id: ProviderId::new(),
        bucket: "archives".into(),
        remote_path: "daily".into(),
        local_path: "/photos".into(),
        mode: SyncMode::Archive,
        keep_last_archives: Some(3),
    }
}

#[test]
fn archive_details_name_the_destination_for_each_direction() {
    let connection = archive_connection();

    assert_eq!(
        archive_details(&connection, Direction::Upload),
        "A new ZIP will be stored in archives/daily. SyncPak will keep the newest 3 remote archives."
    );
    assert_eq!(
        archive_details(&connection, Direction::Download),
        "SyncPak will create a ZIP in /photos from the cloud folder. Remote archive retention does not apply to this download."
    );
}
