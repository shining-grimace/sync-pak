use super::invalidate_changed_connections;
use crate::configuration::{TransportSettings, lists};

#[test]
fn changed_defaults_invalidate_inherited_connections_but_not_explicit_overrides() {
    let mut config = lists::tests::config();
    let mut independent = config.connections[0].clone();
    independent.transport = Some(TransportSettings {
        endpoint: None,
        region: Some("eu-west-1".into()),
    });
    config.connections.push(independent);
    let previous = config.providers[0].clone();
    let mut replacement = previous.clone();
    replacement.options.region = Some("us-west-2".into());
    invalidate_changed_connections(&mut config, &previous, &replacement, false);
    assert!(!config.connections[0].verified);
    assert!(config.connections[1].verified);
}

#[test]
fn changed_credentials_invalidate_all_dependent_connections() {
    let mut config = lists::tests::config();
    config.connections[0].transport = Some(TransportSettings {
        endpoint: None,
        region: Some("eu-west-1".into()),
    });
    let previous = config.providers[0].clone();
    invalidate_changed_connections(&mut config, &previous, &previous, true);
    assert!(!config.connections[0].verified);
}

#[test]
fn display_name_changes_keep_existing_checks() {
    let mut config = lists::tests::config();
    let previous = config.providers[0].clone();
    let mut replacement = previous.clone();
    replacement.name = "Renamed".into();
    invalidate_changed_connections(&mut config, &previous, &replacement, false);
    assert!(config.connections[0].verified);
}

#[test]
fn bucket_specific_routing_changes_invalidate_only_that_bucket() {
    let mut config = lists::tests::config();
    let mut other = config.connections[0].clone();
    other.bucket = "another-bucket".into();
    config.connections.push(other);
    let previous = config.providers[0].clone();
    let mut replacement = previous.clone();
    replacement.options.default_bucket_transport = Some(TransportSettings {
        endpoint: None,
        region: Some("eu-west-1".into()),
    });
    invalidate_changed_connections(&mut config, &previous, &replacement, false);
    assert!(!config.connections[0].verified);
    assert!(config.connections[1].verified);
}
