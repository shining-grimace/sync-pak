use super::paths::local_relative;
use super::{ListDocument, MAX_DOCUMENT_BYTES, PortableConnection, RemotePath, local_path};
use crate::configuration::{AppConfig, ConnectionConfig, ConnectionId, ProviderConfig, ProviderId};
use std::collections::BTreeMap;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum ConflictChoice {
    #[default]
    Skip,
    Replace,
    Copy,
}

pub fn portable(config: &AppConfig, c: &ConnectionConfig) -> Result<PortableConnection, String> {
    let local = local_relative(&config.local_root, &c.local_path).ok_or_else(|| {
        format!(
            "Incompatible: {} is outside Local Root ({})",
            c.local_path, config.local_root
        )
    })?;
    local_path(&config.local_root, &local).map_err(|error| format!("Incompatible: {error}"))?;
    let provider = config
        .providers
        .iter()
        .find(|p| p.id == c.provider_id)
        .ok_or("Provider no longer exists.")?;
    let connection = PortableConnection {
        id: c.id.clone(),
        name: c.name.clone(),
        local_path: local,
        remote: RemotePath {
            provider_kind: provider.kind,
            bucket: c.bucket.clone(),
            path: c.remote_path.trim_matches('/').into(),
        },
        mode: c.mode,
        allow_upload: c.allow_upload,
        allow_download: c.allow_download,
        keep_last_archives: c.keep_last_archives,
    };
    ListDocument::new(vec![connection.clone()]).validate()?;
    Ok(connection)
}

pub fn export_list(config: &AppConfig, selected: &[ConnectionId]) -> Result<Vec<u8>, String> {
    let connections = config
        .connections
        .iter()
        .filter(|c| selected.contains(&c.id))
        .map(|c| portable(config, c).map_err(|e| format!("{}: {e}", c.name)))
        .collect::<Result<Vec<_>, _>>()?;
    if connections.is_empty() {
        return Err("Select at least one compatible connection to export.".into());
    }
    let bytes =
        serde_json::to_vec_pretty(&ListDocument::new(connections)).map_err(|e| e.to_string())?;
    if bytes.len() > MAX_DOCUMENT_BYTES {
        return Err("Select fewer connections: exported JSON exceeds 4 MiB.".into());
    }
    Ok(bytes)
}

/// Infer by provider kind and bucket. Credentials and endpoints remain device-local.
pub fn provider_candidates<'a>(
    config: &'a AppConfig,
    remote: &RemotePath,
) -> Vec<&'a ProviderConfig> {
    let providers: Vec<_> = config
        .providers
        .iter()
        .filter(|p| p.kind == remote.provider_kind)
        .collect();
    let matches: Vec<_> = providers
        .iter()
        .copied()
        .filter(|p| {
            p.options.default_bucket.as_deref() == Some(&remote.bucket)
                || config
                    .connections
                    .iter()
                    .any(|c| c.provider_id == p.id && c.bucket == remote.bucket)
        })
        .collect();
    if matches.is_empty() {
        providers
    } else {
        matches
    }
}

pub fn import_list(
    config: &AppConfig,
    document: &ListDocument,
    choices: &BTreeMap<String, ConflictChoice>,
    mappings: &BTreeMap<String, ProviderId>,
) -> Result<AppConfig, String> {
    document.validate()?;
    let mut result = config.clone();
    for incoming in &document.connections {
        let existing = result.connections.iter().position(|c| c.id == incoming.id);
        let choice = choices
            .get(incoming.id.as_str())
            .copied()
            .unwrap_or_default();
        if existing.is_some() && choice == ConflictChoice::Skip {
            continue;
        }
        let candidates = provider_candidates(config, &incoming.remote);
        let provider = if let Some(id) = mappings.get(incoming.id.as_str()) {
            candidates
                .iter()
                .find(|p| &p.id == id)
                .copied()
                .ok_or("The selected provider does not match this bucket's provider type.")?
        } else if candidates.len() == 1 {
            candidates[0]
        } else {
            return Err(format!(
                "{}: choose a provider for {:?} / {}.",
                incoming.name, incoming.remote.provider_kind, incoming.remote.bucket
            ));
        };
        let connection = ConnectionConfig {
            id: if existing.is_some() && choice == ConflictChoice::Copy {
                ConnectionId::new()
            } else {
                incoming.id.clone()
            },
            name: incoming.name.clone(),
            provider_id: provider.id.clone(),
            bucket: incoming.remote.bucket.clone(),
            local_path: local_path(&config.local_root, &incoming.local_path)?,
            remote_path: incoming.remote.path.clone(),
            mode: incoming.mode,
            allow_upload: incoming.allow_upload,
            allow_download: incoming.allow_download,
            keep_last_archives: incoming.keep_last_archives,
            verified: false,
        };
        if let Some(index) = existing.filter(|_| choice == ConflictChoice::Replace) {
            result.connections[index] = connection;
        } else {
            result.connections.push(connection);
        }
    }
    result.validate().map_err(|e| e.to_string())?;
    Ok(result)
}
