use super::*;

#[test]
fn settings_persist_one_root_and_infer_relative_connections() {
    let config = config();
    let bytes = encode(&config).unwrap();
    let json: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
    assert_eq!(json["schema_version"], 1);
    assert_eq!(json["local_root"], config.local_root);
    assert_eq!(json["connections"][0]["local_path"], "Photos");
    assert!(json.get("connection_lists").is_none());
    assert_eq!(decode(&bytes).unwrap(), config);
}

#[test]
fn changing_root_keeps_configured_destinations_and_round_trips_incompatible_paths() {
    let mut config = config();
    let original = config.connections.clone();
    config.local_root = std::env::temp_dir()
        .join("new-root")
        .to_str()
        .unwrap()
        .into();
    let bytes = encode(&config).unwrap();
    let loaded = decode(&bytes).unwrap();
    assert_eq!(loaded.connections, original);
    assert_eq!(loaded.local_root, config.local_root);
    assert!(portable(&loaded, &loaded.connections[0]).is_err());
}

#[test]
fn malformed_absolute_and_traversing_imports_are_rejected() {
    let config = config();
    let mut document = document(&config);
    for path in ["../secret", "/absolute", "C:/Windows", "a\\b"] {
        document.connections[0].local_path = path.into();
        assert!(document.validate().is_err());
    }
    assert!(ListDocument::parse(&vec![b' '; MAX_DOCUMENT_BYTES + 1]).is_err());
    assert!(decode(br#"{"schema_version":4}"#).is_err());
}

#[test]
fn initial_root_uses_the_home_directory() {
    #[cfg(target_os = "windows")]
    let home = std::env::var("USERPROFILE").unwrap();
    #[cfg(not(target_os = "windows"))]
    let home = std::env::var("HOME").unwrap();
    assert_eq!(AppConfig::default().local_root, home);
}

#[test]
fn failed_save_preserves_existing_file() {
    let directory = std::env::temp_dir().join(uuid::Uuid::new_v4().to_string());
    let store = crate::configuration::ConfigStore::at(directory.join("config.json"));
    let mut config = config();
    store.save(&config).unwrap();
    let original = std::fs::read(store.path()).unwrap();
    config.local_root = "not-an-absolute-root".into();
    assert!(store.save(&config).is_err());
    assert_eq!(std::fs::read(store.path()).unwrap(), original);
    std::fs::remove_dir_all(directory).unwrap();
}
