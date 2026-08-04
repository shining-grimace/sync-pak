use serde::{Deserialize, Serialize};
use uuid::Uuid;

use super::validation::ValidationErrors;

pub const CURRENT_SCHEMA_VERSION: u32 = 4;

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct AppConfig {
    pub schema_version: u32,
    pub welcome_completed: bool,
    pub appearance: AppearancePreference,
    pub providers: Vec<ProviderConfig>,
    pub connections: Vec<ConnectionConfig>,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            schema_version: CURRENT_SCHEMA_VERSION,
            welcome_completed: false,
            appearance: AppearancePreference::System,
            providers: Vec::new(),
            connections: Vec::new(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum AppearancePreference {
    #[default]
    System,
    Light,
    Dark,
}

impl AppearancePreference {
    pub(crate) fn from_index(index: i32) -> Option<Self> {
        match index {
            0 => Some(Self::System),
            1 => Some(Self::Light),
            2 => Some(Self::Dark),
            _ => None,
        }
    }

    pub(crate) fn index(self) -> i32 {
        match self {
            Self::System => 0,
            Self::Light => 1,
            Self::Dark => 2,
        }
    }
}

impl AppConfig {
    pub fn validate(&self) -> Result<(), ValidationErrors> {
        super::validation::validate(self)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProviderId(String);

impl ProviderId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ProviderId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ConnectionId(String);

impl ConnectionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4().to_string())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct CredentialReference {
    pub provider_id: ProviderId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderConfig {
    pub id: ProviderId,
    pub name: String,
    pub kind: ProviderKind,
    pub options: ProviderOptions,
    pub credential_reference: CredentialReference,
    pub verified: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ProviderKind {
    CloudflareR2,
    BackblazeB2,
    AwsS3,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProviderOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub account_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_bucket: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub endpoint: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub id: ConnectionId,
    pub name: String,
    pub provider_id: ProviderId,
    pub bucket: String,
    pub remote_path: String,
    pub local_path: String,
    pub mode: SyncMode,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub keep_last_archives: Option<u32>,
    pub verified: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectionDraft {
    pub name: String,
    pub provider_id: ProviderId,
    pub bucket: String,
    pub remote_path: String,
    pub local_path: String,
    pub mode: SyncMode,
    pub keep_last_archives: Option<u32>,
    pub verified: bool,
}

impl ConnectionDraft {
    pub fn into_config(self, id: ConnectionId) -> ConnectionConfig {
        ConnectionConfig {
            id,
            name: self.name,
            provider_id: self.provider_id,
            bucket: self.bucket,
            remote_path: self.remote_path,
            local_path: self.local_path,
            mode: self.mode,
            keep_last_archives: self.keep_last_archives,
            verified: self.verified,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SyncMode {
    AddOnly,
    Mirror,
    Archive,
}
