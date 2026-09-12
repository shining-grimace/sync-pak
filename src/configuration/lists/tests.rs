use super::*;
use crate::configuration::{
    AppConfig, ConnectionConfig, ConnectionId, CredentialReference, ProviderConfig, ProviderId,
    ProviderKind, ProviderOptions, SyncMode,
};
use std::collections::BTreeMap;

mod paths;
mod persistence;

pub(crate) fn config() -> AppConfig {
    let id = ProviderId::new();
    let provider = ProviderConfig {
        id: id.clone(),
        name: "Cloud".into(),
        kind: ProviderKind::AwsS3,
        options: ProviderOptions {
            account_id: None,
            endpoint: None,
            region: Some("ap-southeast-2".into()),
            default_bucket: Some("photos-backup".into()),
        },
        credential_reference: CredentialReference {
            provider_id: id.clone(),
        },
        verified: true,
    };
    let root = std::env::temp_dir().join("syncpak-local-root");
    let connection = ConnectionConfig {
        id: ConnectionId::new(),
        name: "Photos".into(),
        provider_id: id,
        bucket: "photos-backup".into(),
        local_path: root.join("Photos").to_str().unwrap().into(),
        remote_path: "photos".into(),
        mode: SyncMode::Mirror,
        allow_upload: true,
        allow_download: false,
        keep_last_archives: None,
        verified: true,
    };
    AppConfig {
        providers: vec![provider],
        connections: vec![connection],
        local_root: root.to_str().unwrap().into(),
        ..Default::default()
    }
}
fn document(config: &AppConfig) -> ListDocument {
    ListDocument::parse(
        &export_list(config, std::slice::from_ref(&config.connections[0].id)).unwrap(),
    )
    .unwrap()
}

#[test]
fn export_infers_relative_paths_and_omits_device_provider_details() {
    let config = config();
    let bytes = export_list(&config, std::slice::from_ref(&config.connections[0].id)).unwrap();
    let text = String::from_utf8(bytes.clone()).unwrap();
    for private in [
        "credential",
        "verified",
        "account_id",
        "region",
        "endpoint",
        "local_root",
        "root_id",
        "buckets",
        config.providers[0].id.as_str(),
        &config.local_root,
    ] {
        assert!(!text.contains(private), "{private}");
    }
    let document = ListDocument::parse(&bytes).unwrap();
    assert_eq!(document.connections[0].local_path, "Photos");
    assert_eq!(document.connections[0].remote.bucket, "photos-backup");
    assert_eq!(
        document.connections[0].remote.provider_kind,
        ProviderKind::AwsS3
    );
    assert_eq!(document.connections[0].id, config.connections[0].id);
}

#[test]
fn another_device_matches_type_and_bucket_despite_different_credentials_metadata() {
    let source = config();
    let mut target = config();
    target.connections.clear();
    target.local_root = std::env::temp_dir()
        .join("another-device")
        .to_str()
        .unwrap()
        .into();
    target.providers[0].name = "Different display name".into();
    target.providers[0].options.region = Some("us-east-1".into());
    target.providers[0].options.endpoint = Some("https://device.example".into());
    let result = import_list(
        &target,
        &document(&source),
        &BTreeMap::new(),
        &BTreeMap::new(),
    )
    .unwrap();
    assert_eq!(result.connections[0].provider_id, target.providers[0].id);
    assert_eq!(
        result.connections[0].local_path,
        std::path::Path::new(&target.local_root)
            .join("Photos")
            .to_str()
            .unwrap()
    );
    assert!(!result.connections[0].verified);
}

#[test]
fn repeated_import_defaults_to_skip_without_modifying_existing_settings() {
    let config = config();
    assert_eq!(
        import_list(
            &config,
            &document(&config),
            &BTreeMap::new(),
            &BTreeMap::new()
        )
        .unwrap(),
        config
    );
}

#[test]
fn replace_and_copy_are_explicit_and_unverified() {
    for choice in [ConflictChoice::Replace, ConflictChoice::Copy] {
        let config = config();
        let mut incoming = document(&config);
        incoming.connections[0].name = "Imported".into();
        let choices = BTreeMap::from([(config.connections[0].id.as_str().into(), choice)]);
        let result = import_list(&config, &incoming, &choices, &BTreeMap::new()).unwrap();
        assert_eq!(
            result.connections.len(),
            if choice == ConflictChoice::Copy { 2 } else { 1 }
        );
        let c = result
            .connections
            .iter()
            .find(|c| c.name == "Imported")
            .unwrap();
        assert!(!c.verified);
        assert_eq!(
            c.id == config.connections[0].id,
            choice == ConflictChoice::Replace
        );
    }
}

#[test]
fn outside_root_is_incompatible_but_can_be_excluded_from_export() {
    let mut config = config();
    let mut outside = config.connections[0].clone();
    outside.id = ConnectionId::new();
    outside.local_path = std::env::temp_dir()
        .join("outside")
        .to_str()
        .unwrap()
        .into();
    let outside_id = outside.id.clone();
    config.connections.push(outside);
    assert!(
        export_list(&config, &[outside_id])
            .unwrap_err()
            .contains("Incompatible:")
    );
    assert_eq!(document(&config).connections.len(), 1);
}

#[test]
fn ambiguous_provider_requires_choice_and_wrong_provider_type_is_rejected() {
    let mut config = config();
    let mut other = config.providers[0].clone();
    other.id = ProviderId::new();
    other.credential_reference.provider_id = other.id.clone();
    config.providers.push(other);
    let mut incoming = document(&config);
    incoming.connections[0].id = ConnectionId::new();
    assert!(import_list(&config, &incoming, &BTreeMap::new(), &BTreeMap::new()).is_err());
    let mappings = BTreeMap::from([(
        incoming.connections[0].id.as_str().into(),
        config.providers[1].id.clone(),
    )]);
    assert!(import_list(&config, &incoming, &BTreeMap::new(), &mappings).is_ok());
    incoming.connections[0].remote.provider_kind = ProviderKind::CloudflareR2;
    assert!(import_list(&config, &incoming, &BTreeMap::new(), &mappings).is_err());
}
