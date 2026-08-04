use crate::{
    configuration::{ConnectionConfig, ConnectionId, ProviderId, SyncMode},
    planning::Direction,
};

use crate::run_direction_presentation::{archive_details, remote_endpoint};

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
        verified: false,
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

#[test]
fn remote_endpoint_names_the_provider_bucket_and_folder() {
    let connection = archive_connection();

    assert_eq!(
        remote_endpoint("Personal cloud", &connection),
        "Personal cloud · archives/daily"
    );

    let mut root_connection = connection;
    root_connection.remote_path.clear();
    assert_eq!(
        remote_endpoint("Personal cloud", &root_connection),
        "Personal cloud · archives (bucket root)"
    );
}
