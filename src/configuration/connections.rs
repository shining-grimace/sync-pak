use super::{
    ConfigStore, ConfigurationError, ConnectionConfig, ConnectionId, ProviderId, SyncMode,
    TransportSettings,
};

#[derive(Debug)]
pub enum ConnectionError {
    Configuration(ConfigurationError),
    NotFound,
}

impl std::fmt::Display for ConnectionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Configuration(error) => {
                write!(formatter, "Connection settings were not saved: {error}")
            }
            Self::NotFound => formatter.write_str("The connection no longer exists."),
        }
    }
}

impl std::error::Error for ConnectionError {}

pub struct ConnectionRepository<'a> {
    configuration: &'a ConfigStore,
}

impl<'a> ConnectionRepository<'a> {
    pub fn new(configuration: &'a ConfigStore) -> Self {
        Self { configuration }
    }

    pub fn create(&self, draft: ConnectionDraft) -> Result<ConnectionConfig, ConnectionError> {
        let connection = draft.into_config(ConnectionId::new());
        let mut config = self.load()?;
        config.connections.push(connection.clone());
        self.save(&config)?;
        Ok(connection)
    }

    pub fn update(
        &self,
        id: &ConnectionId,
        draft: ConnectionDraft,
    ) -> Result<ConnectionConfig, ConnectionError> {
        let mut config = self.load()?;
        let mut connection = draft.into_config(id.clone());
        let existing = config
            .connections
            .iter_mut()
            .find(|item| &item.id == id)
            .ok_or(ConnectionError::NotFound)?;
        connection.last_run = existing.last_run;
        *existing = connection.clone();
        self.save(&config)?;
        Ok(connection)
    }

    pub fn delete(&self, id: &ConnectionId) -> Result<ConnectionConfig, ConnectionError> {
        let mut config = self.load()?;
        let index = config
            .connections
            .iter()
            .position(|connection| &connection.id == id)
            .ok_or(ConnectionError::NotFound)?;
        let connection = config.connections.remove(index);
        self.save(&config)?;
        Ok(connection)
    }

    fn load(&self) -> Result<super::AppConfig, ConnectionError> {
        self.configuration
            .load()
            .map_err(ConnectionError::Configuration)
    }

    fn save(&self, config: &super::AppConfig) -> Result<(), ConnectionError> {
        self.configuration
            .save(config)
            .map_err(ConnectionError::Configuration)
    }
}

impl ConfigStore {
    pub(crate) fn record_connection_run(
        &self,
        connection_id: &str,
        timestamp: u64,
    ) -> Result<bool, ConfigurationError> {
        let mut config = self.load()?;
        let Some(connection) = config
            .connections
            .iter_mut()
            .find(|connection| connection.id.as_str() == connection_id)
        else {
            return Ok(false);
        };
        connection.last_run = Some(timestamp);
        self.save(&config)?;
        Ok(true)
    }

    pub(crate) fn record_connection_verification(
        &self,
        connection_id: &str,
    ) -> Result<bool, ConfigurationError> {
        let mut config = self.load()?;
        let Some(connection) = config
            .connections
            .iter_mut()
            .find(|connection| connection.id.as_str() == connection_id)
        else {
            return Ok(false);
        };
        connection.verified = true;
        self.save(&config)?;
        Ok(true)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConnectionDraft {
    pub name: String,
    pub provider_id: ProviderId,
    pub bucket: String,
    pub transport: Option<TransportSettings>,
    pub remote_path: String,
    pub local_path: String,
    pub mode: SyncMode,
    pub allow_upload: bool,
    pub allow_download: bool,
    pub show_preview: bool,
    pub keep_last_archives: Option<u32>,
    pub verified: bool,
}

impl ConnectionDraft {
    pub fn into_config(self, id: ConnectionId) -> ConnectionConfig {
        ConnectionConfig {
            last_run: None,
            id,
            name: self.name,
            provider_id: self.provider_id,
            bucket: self.bucket,
            transport: self.transport,
            remote_path: self.remote_path,
            local_path: self.local_path,
            mode: self.mode,
            allow_upload: self.allow_upload,
            allow_download: self.allow_download,
            show_preview: self.show_preview,
            keep_last_archives: self.keep_last_archives,
            verified: self.verified,
        }
    }
}
