use crate::{
    configuration::{
        AppConfig, ConnectionConfig, ConnectionId, ProviderConfig, ProviderId, SyncMode,
    },
    planning::Direction,
};

use super::{RunRequest, RunRequestError};

fn config(mode: SyncMode) -> AppConfig {
    let provider_id = ProviderId::new();
    let provider = ProviderConfig {
        id: provider_id.clone(),
        name: "Cloud".into(),
        kind: crate::configuration::ProviderKind::AwsS3,
        options: crate::configuration::ProviderOptions {
            account_id: None,
            default_bucket: None,
            endpoint: None,
            region: Some("ap-southeast-2".into()),
        },
        credential_reference: crate::configuration::CredentialReference { provider_id },
    };
    AppConfig {
        connections: vec![ConnectionConfig {
            id: ConnectionId::new(),
            name: "Photos".into(),
            provider_id: provider.id.clone(),
            bucket: "backups".into(),
            remote_path: String::new(),
            local_path: "/photos".into(),
            mode,
            keep_last_archives: None,
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
