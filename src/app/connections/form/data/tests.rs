use crate::configuration::{
    CredentialReference, ProviderConfig, ProviderId, ProviderKind, ProviderOptions,
};

use super::provider_bucket;

fn provider(default_bucket: Option<&str>) -> ProviderConfig {
    let id = ProviderId::new();
    ProviderConfig {
        id: id.clone(),
        credential_reference: CredentialReference { provider_id: id },
        name: "Cloud".into(),
        kind: ProviderKind::AwsS3,
        options: ProviderOptions {
            default_bucket_transport: None,
            account_id: None,
            default_bucket: default_bucket.map(str::to_owned),
            endpoint: None,
            region: Some("ap-southeast-2".into()),
        },
        verified: false,
    }
}

#[test]
fn provider_default_bucket_is_available_only_for_a_valid_selection() {
    let providers = [provider(Some("new-default")), provider(None)];

    assert_eq!(provider_bucket(&providers, 0), Some("new-default"));
    assert_eq!(provider_bucket(&providers, 1), None);
    assert_eq!(provider_bucket(&providers, -1), None);
}

#[test]
fn drafts_keep_explicit_transport_settings_from_the_form() {
    let mut config = crate::configuration::lists::tests::config();
    config.connections[0].transport = Some(crate::configuration::TransportSettings {
        endpoint: None,
        region: Some("eu-west-1".into()),
    });
    let directory = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let store = crate::configuration::ConfigStore::at(directory.join("config.json"));
    store.save(&config).unwrap();
    let connection = &config.connections[0];
    for bucket in [&*connection.bucket, "another-bucket"] {
        let draft = super::draft(
            &store,
            connection.transport.clone(),
            "Renamed".into(),
            0,
            bucket.into(),
            connection.remote_path.clone().into(),
            connection.local_path.clone().into(),
            1,
            "".into(),
            true,
            false,
            true,
        )
        .unwrap();
        assert_eq!(draft.transport, connection.transport);
    }
    std::fs::remove_dir_all(directory).unwrap();
}
