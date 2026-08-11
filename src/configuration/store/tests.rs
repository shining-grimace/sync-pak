use super::ConfigStore;
use crate::configuration::{AppConfig, AppearancePreference, CURRENT_SCHEMA_VERSION};
use std::{
    fs,
    time::{SystemTime, UNIX_EPOCH},
};

#[test]
fn saves_and_loads_a_versioned_configuration() {
    let path = test_path("config");
    let store = ConfigStore::at(path.clone());
    store.save(&AppConfig::default()).unwrap();
    assert_eq!(store.load().unwrap(), AppConfig::default());
    assert!(fs::read_to_string(path).unwrap().contains("schema_version"));
}

#[test]
fn saves_and_loads_the_appearance_preference() {
    let path = test_path("appearance");
    let store = ConfigStore::at(path.clone());
    let config = AppConfig {
        appearance: AppearancePreference::Dark,
        ..Default::default()
    };

    store.save(&config).unwrap();

    assert_eq!(store.load().unwrap().appearance, AppearancePreference::Dark);
    assert!(fs::read_to_string(path).unwrap().contains("appearance"));
}

#[test]
fn rejects_an_unversioned_configuration() {
    let path = test_path("unversioned");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, "{}").unwrap();

    assert!(ConfigStore::at(path).load().is_err());
}

#[test]
fn rejects_an_older_schema() {
    let path = test_path("older-schema");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(
        &path,
        r#"{"schema_version":3,"welcome_completed":false,"appearance":"system","providers":[],"connections":[]}"#,
    )
    .unwrap();

    assert!(ConfigStore::at(path).load().is_err());
    assert_eq!(CURRENT_SCHEMA_VERSION, 4);
}

#[test]
fn save_does_not_overwrite_a_stale_temporary_file() {
    let path = test_path("stale-temporary");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let stale = path.parent().unwrap().join(".config.json.tmp");
    fs::write(&stale, "stale").unwrap();

    ConfigStore::at(path).save(&AppConfig::default()).unwrap();

    assert_eq!(fs::read_to_string(stale).unwrap(), "stale");
}

fn test_path(label: &str) -> std::path::PathBuf {
    std::env::temp_dir()
        .join(format!(
            "sync-pak-{label}-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
        .join("config.json")
}
