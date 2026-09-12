use crate::configuration::{ConnectionId, ProviderKind, SyncMode};
use serde::{Deserialize, Serialize};

pub const MAX_DOCUMENT_BYTES: usize = 4 * 1024 * 1024;

/// Remote identity is derived from these two values, independently of device credentials.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RemotePath {
    pub provider_kind: ProviderKind,
    pub bucket: String,
    pub path: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PortableConnection {
    pub id: ConnectionId,
    pub name: String,
    pub local_path: String,
    pub remote: RemotePath,
    pub mode: SyncMode,
    pub allow_upload: bool,
    pub allow_download: bool,
    pub keep_last_archives: Option<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ListDocument {
    pub format: String,
    pub version: u32,
    pub connections: Vec<PortableConnection>,
}
impl ListDocument {
    pub fn new(connections: Vec<PortableConnection>) -> Self {
        Self {
            format: "syncpak-connection-list".into(),
            version: 1,
            connections,
        }
    }
    pub fn parse(bytes: &[u8]) -> Result<Self, String> {
        if bytes.len() > MAX_DOCUMENT_BYTES {
            return Err("Connection list exceeds 4 MiB.".into());
        }
        let document: Self = serde_json::from_slice(bytes)
            .map_err(|e| format!("Invalid connection list JSON: {e}"))?;
        document.validate()?;
        Ok(document)
    }
    pub fn validate(&self) -> Result<(), String> {
        super::validation::validate(self)
    }
}
