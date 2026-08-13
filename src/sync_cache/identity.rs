use std::path::Path;

use uuid::Uuid;

use crate::{operations::request::RunRequest, platform::atomic_write::atomic_write};

const INSTALLATION_FILE: &str = "installation-id";

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct CacheNamespace(String);

impl CacheNamespace {
    pub fn new(installation_id: &str, request: &RunRequest) -> Self {
        let provider = &request.provider;
        let connection = &request.connection;
        Self(format!(
            "v1\ninstallation={installation_id}\nprovider={}\nkind={:?}\nendpoint={:?}\nregion={:?}\naccount={:?}\nbucket={}\nprefix={}\nconnection={}\nlocal={}",
            provider.id.as_str(),
            provider.kind,
            provider.options.endpoint,
            provider.options.region,
            provider.options.account_id,
            connection.bucket,
            connection.remote_path,
            connection.id.as_str(),
            connection.local_path,
        ))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }

    #[cfg(test)]
    pub(crate) fn from_stored(value: String) -> Self {
        Self(value)
    }
}

pub fn installation_id(directory: &Path) -> Option<String> {
    let path = directory.join(INSTALLATION_FILE);
    if let Ok(value) = std::fs::read_to_string(&path) {
        let value = value.trim();
        if Uuid::parse_str(value).is_ok() {
            return Some(value.to_owned());
        }
    }
    let value = Uuid::new_v4().to_string();
    atomic_write(&path, value.as_bytes()).ok()?;
    Some(value)
}
