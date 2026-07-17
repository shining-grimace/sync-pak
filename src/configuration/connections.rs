use super::{ConfigStore, ConfigurationError, ConnectionConfig, ConnectionDraft, ConnectionId};

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
        let connection = draft.into_config(id.clone());
        let existing = config
            .connections
            .iter_mut()
            .find(|item| &item.id == id)
            .ok_or(ConnectionError::NotFound)?;
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
