use crate::{
    configuration::{
        AppConfig, ConnectionConfig, ConnectionId, ProviderConfig, ProviderId, SyncMode,
    },
    preflight::planning::Direction,
};

use super::{RunRequest, RunRequestError};

fn config(mode: SyncMode) -> AppConfig {
    let provider_id = ProviderId::new();
    let provider = ProviderConfig {
        id: provider_id.clone(),
        name: "Cloud".into(),
        kind: crate::configuration::ProviderKind::AwsS3,
        options: crate::configuration::ProviderOptions {
            default_bucket_transport: None,
            account_id: None,
            default_bucket: None,
            endpoint: None,
            region: Some("ap-southeast-2".into()),
        },
        credential_reference: crate::configuration::CredentialReference { provider_id },
        verified: true,
    };
    AppConfig {
        connections: vec![ConnectionConfig {
            last_run: None,
            transport: None,
            id: ConnectionId::new(),
            name: "Photos".into(),
            provider_id: provider.id.clone(),
            bucket: "backups".into(),
            remote_path: String::new(),
            local_path: "/photos".into(),
            mode,
            allow_upload: true,
            allow_download: true,
            show_preview: true,
            keep_last_archives: None,
            verified: true,
        }],
        providers: vec![provider],
        ..Default::default()
    }
}

#[test]
fn builds_a_non_secret_request_for_a_current_connection() {
    let config = config(SyncMode::AddOnly);
    let request = RunRequest::from_config(
        &config,
        config.connections[0].id.as_str(),
        Direction::BothWays,
    )
    .unwrap();

    assert_eq!(request.connection.name, "Photos");
    assert_eq!(request.provider.name, "Cloud");
    assert_eq!(request.direction, Direction::BothWays);
}

#[test]
fn rejects_both_ways_for_non_additive_connections() {
    let config = config(SyncMode::Mirror);
    assert_eq!(
        RunRequest::from_config(
            &config,
            config.connections[0].id.as_str(),
            Direction::BothWays,
        ),
        Err(RunRequestError::BothWaysUnsupported)
    );
}

#[test]
fn rejects_directions_disabled_by_connection_settings() {
    let mut config = config(SyncMode::AddOnly);
    let connection_id = config.connections[0].id.as_str().to_owned();
    config.connections[0].allow_upload = false;

    assert_eq!(
        RunRequest::from_config(&config, &connection_id, Direction::Upload),
        Err(RunRequestError::DirectionNotAllowed)
    );
    assert_eq!(
        RunRequest::from_config(&config, &connection_id, Direction::BothWays),
        Err(RunRequestError::DirectionNotAllowed)
    );

    config.connections[0].allow_download = false;
    assert_eq!(
        RunRequest::from_config(&config, &connection_id, Direction::Download),
        Err(RunRequestError::DirectionNotAllowed)
    );
}

#[test]
fn automatic_runs_resolve_allowed_directions_and_require_saved_consent() {
    let mut config = config(SyncMode::AddOnly);
    let id = config.connections[0].id.as_str().to_owned();
    assert_eq!(
        RunRequest::automatic(&config, &id),
        Err(RunRequestError::PreviewRequired)
    );
    config.connections[0].show_preview = false;
    assert_eq!(
        RunRequest::automatic(&config, &id).unwrap().direction,
        Direction::BothWays
    );
    config.connections[0].allow_upload = false;
    assert_eq!(
        RunRequest::automatic(&config, &id).unwrap().direction,
        Direction::Download
    );
    config.connections[0].allow_upload = true;
    config.connections[0].allow_download = false;
    assert_eq!(
        RunRequest::automatic(&config, &id).unwrap().direction,
        Direction::Upload
    );
    config.connections[0].verified = false;
    assert_eq!(
        RunRequest::automatic(&config, &id),
        Err(RunRequestError::NotVerified)
    );
}
