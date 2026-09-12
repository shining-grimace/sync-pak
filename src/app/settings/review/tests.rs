use super::*;
use crate::configuration::{AppearancePreference, lists::tests::config};

fn store(config: &AppConfig) -> ConfigStore {
    let store = ConfigStore::at(
        std::env::temp_dir()
            .join(uuid::Uuid::new_v4().to_string())
            .join("config.json"),
    );
    store.save(config).unwrap();
    store
}
fn clean(store: &ConfigStore) {
    std::fs::remove_dir_all(store.path().parent().unwrap()).unwrap();
}

#[test]
fn root_review_is_global_and_works_without_connections() {
    let mut config = config();
    config.connections.clear();
    let store = store(&config);
    let mut review = Review::new(config, None, 3);
    review.local_root = std::env::temp_dir()
        .join("chosen-root")
        .to_str()
        .unwrap()
        .into();
    assert_eq!(review.commit(&store).unwrap(), 0);
    assert_eq!(store.load().unwrap().local_root, review.local_root);
    clean(&store);
}

#[test]
fn cancellation_and_invalid_root_leave_settings_unchanged() {
    let config = config();
    let store = store(&config);
    let bytes = std::fs::read(store.path()).unwrap();
    let mut review = Review::new(config, None, 3);
    review.local_root = "relative/root".into();
    assert!(review.commit(&store).is_err());
    drop(review);
    assert_eq!(std::fs::read(store.path()).unwrap(), bytes);
    clean(&store);
}

#[test]
fn root_edits_preserve_existing_connections_and_verification() {
    let config = config();
    let store = store(&config);
    let mut review = Review::new(config.clone(), None, 3);
    review.local_root = std::env::temp_dir()
        .join("another-root")
        .to_str()
        .unwrap()
        .into();
    review.commit(&store).unwrap();
    assert_eq!(store.load().unwrap().connections, config.connections);
    clean(&store);
}

#[test]
fn concurrent_settings_changes_are_not_overwritten() {
    let config = config();
    let store = store(&config);
    let review = Review::new(config.clone(), None, 3);
    let mut changed = config;
    changed.appearance = AppearancePreference::Dark;
    store.save(&changed).unwrap();
    assert!(review.commit(&store).is_err());
    assert_eq!(store.load().unwrap().appearance, AppearancePreference::Dark);
    clean(&store);
}

#[test]
fn documented_json_example_is_valid() {
    let document = ListDocument::parse(include_bytes!(
        "../../../../docs/examples/connection-list.json"
    ))
    .unwrap();
    assert_eq!(document.connections[0].local_path, "Projects");
}
