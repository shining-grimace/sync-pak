use super::paths::{local_path, local_relative};
use crate::configuration::{AppConfig, AppearancePreference, ConnectionConfig, ProviderConfig};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct SettingsFile {
    schema_version: u32,
    welcome_completed: bool,
    appearance: AppearancePreference,
    #[serde(default)]
    connection_filter: i32,
    #[serde(default)]
    connections_newest_first: bool,
    local_root: String,
    providers: Vec<ProviderConfig>,
    connections: Vec<ConnectionConfig>,
}

pub(crate) fn encode(config: &AppConfig) -> Result<Vec<u8>, String> {
    local_path(&config.local_root, "")?;
    let mut connections = config.connections.clone();
    for connection in &mut connections {
        if let Some(relative) = local_relative(&config.local_root, &connection.local_path)
            .filter(|relative| local_path(&config.local_root, relative).is_ok())
        {
            connection.local_path = relative;
        } else if !absolute_local(&connection.local_path) {
            return Err("A configured connection must have an absolute local destination.".into());
        }
    }
    let file = SettingsFile {
        schema_version: config.schema_version,
        welcome_completed: config.welcome_completed,
        appearance: config.appearance,
        connection_filter: config.connection_filter,
        connections_newest_first: config.connections_newest_first,
        local_root: config.local_root.clone(),
        providers: config.providers.clone(),
        connections,
    };
    serde_json::to_vec_pretty(&file).map_err(|e| e.to_string())
}

pub(crate) fn decode(bytes: &[u8]) -> Result<AppConfig, String> {
    let file: SettingsFile =
        serde_json::from_slice(bytes).map_err(|e| format!("Invalid settings JSON: {e}"))?;
    if file.schema_version != 1 {
        return Err("Unsupported settings schema. This build requires version 1; previous developer settings must be removed manually.".into());
    }
    local_path(&file.local_root, "")?;
    let mut connections = file.connections;
    for connection in &mut connections {
        if !absolute_local(&connection.local_path) {
            connection.local_path = local_path(&file.local_root, &connection.local_path)?;
        }
    }
    let config = AppConfig {
        schema_version: file.schema_version,
        welcome_completed: file.welcome_completed,
        appearance: file.appearance,
        connection_filter: file.connection_filter.clamp(0, 3),
        connections_newest_first: file.connections_newest_first,
        local_root: file.local_root,
        providers: file.providers,
        connections,
    };
    config.validate().map_err(|e| e.to_string())?;
    Ok(config)
}
fn absolute_local(path: &str) -> bool {
    std::path::Path::new(path).is_absolute() || path.starts_with("content://")
}
