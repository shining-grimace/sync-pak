use super::TransportSettings;
use crate::configuration::{ConnectionId, lists};

fn settings(endpoint: Option<&str>, region: Option<&str>) -> TransportSettings {
    TransportSettings {
        endpoint: endpoint.map(str::to_owned),
        region: region.map(str::to_owned),
    }
}

#[test]
fn bucket_defaults_apply_only_to_the_matching_bucket() {
    let mut config = lists::tests::config();
    let options = &mut config.providers[0].options;
    let account = options.transport_for_bucket(None, None);
    let bucket = settings(Some("https://bucket-endpoint.example"), Some("us-west-2"));
    options.default_bucket_transport = Some(bucket.clone());
    assert_eq!(
        options.transport_for_bucket(Some("photos-backup"), None),
        bucket
    );
    assert_eq!(
        options.transport_for_bucket(Some("other-bucket"), None),
        account
    );
    assert_eq!(options.transport_for_bucket(None, None), account);
    options.default_bucket = Some("new-default".into());
    assert_eq!(
        options.transport_for_bucket(Some("photos-backup"), None),
        account
    );
}

#[test]
fn connection_settings_replace_the_entire_inherited_transport() {
    let mut config = lists::tests::config();
    let options = &mut config.providers[0].options;
    options.endpoint = Some("https://private.example".into());
    options.default_bucket_transport =
        Some(settings(Some("https://bucket.example"), Some("us-west-2")));
    let connection = settings(None, Some("eu-west-1"));
    assert_eq!(
        options.transport_for_bucket(Some("photos-backup"), Some(&connection)),
        connection
    );
}

#[test]
fn overrides_round_trip_through_device_storage_and_connection_drafts() {
    let mut config = lists::tests::config();
    config.providers[0].options.default_bucket_transport =
        Some(settings(Some("https://bucket.example"), Some("us-west-2")));
    config.connections[0].transport = Some(settings(None, Some("eu-west-1")));
    assert_eq!(
        lists::decode(&lists::encode(&config).unwrap()).unwrap(),
        config
    );
    let connection = &config.connections[0];
    let draft = crate::configuration::ConnectionDraft {
        name: connection.name.clone(),
        provider_id: connection.provider_id.clone(),
        bucket: connection.bucket.clone(),
        transport: connection.transport.clone(),
        remote_path: connection.remote_path.clone(),
        local_path: connection.local_path.clone(),
        mode: connection.mode,
        allow_upload: true,
        allow_download: false,
        show_preview: true,
        keep_last_archives: None,
        verified: false,
    };
    assert_eq!(
        draft.into_config(ConnectionId::new()).transport,
        connection.transport
    );
}

#[test]
fn validates_bucket_association_and_rejects_empty_explicit_values() {
    let mut config = lists::tests::config();
    config.providers[0].options.default_bucket = None;
    config.providers[0].options.default_bucket_transport = Some(TransportSettings::default());
    assert!(config.validate().is_err());
    config.providers[0].options.default_bucket_transport = None;
    config.providers[0].options.region = None;
    assert!(
        config.validate().is_ok(),
        "credentials-only providers are valid"
    );
    config.connections[0].transport = Some(settings(None, Some(" ")));
    assert!(config.validate().is_err());
}

#[test]
fn portable_lists_keep_transport_on_the_destination_device() {
    let mut source = lists::tests::config();
    source.connections[0].transport =
        Some(settings(Some("https://source.example"), Some("eu-west-1")));
    let document = lists::ListDocument::parse(
        &lists::export_list(&source, &[source.connections[0].id.clone()]).unwrap(),
    )
    .unwrap();
    let mut target = lists::tests::config();
    target.connections.clear();
    let imported =
        lists::import_list(&target, &document, &Default::default(), &Default::default()).unwrap();
    assert!(imported.connections[0].transport.is_none());
}

#[test]
fn changing_connection_transport_changes_cache_identity() {
    let config = lists::tests::config();
    let mut request = crate::operations::request::RunRequest::from_config(
        &config,
        config.connections[0].id.as_str(),
        crate::preflight::planning::Direction::Upload,
    )
    .unwrap();
    let original = crate::sync_cache::CacheNamespace::new("installation", &request);
    request.connection.transport = Some(settings(None, Some("eu-west-1")));
    assert_ne!(
        original,
        crate::sync_cache::CacheNamespace::new("installation", &request)
    );
}
