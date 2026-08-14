use std::{error::Error, fmt};

use crate::{
    configuration::{AppConfig, ConnectionConfig, ProviderConfig, SyncMode},
    preflight::planning::Direction,
};

/// A non-secret, immutable handoff from direction selection to inventory/preflight work.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RunRequest {
    pub connection: ConnectionConfig,
    pub provider: ProviderConfig,
    pub direction: Direction,
}

impl RunRequest {
    pub fn from_config(
        config: &AppConfig,
        connection_id: &str,
        direction: Direction,
    ) -> Result<Self, RunRequestError> {
        let connection = config
            .connections
            .iter()
            .find(|connection| connection.id.as_str() == connection_id)
            .cloned()
            .ok_or(RunRequestError::ConnectionNotFound)?;
        let provider = config
            .providers
            .iter()
            .find(|provider| provider.id == connection.provider_id)
            .cloned()
            .ok_or(RunRequestError::ProviderNotFound)?;
        if direction == Direction::BothWays && connection.mode != SyncMode::AddOnly {
            return Err(RunRequestError::BothWaysUnsupported);
        }
        if (direction == Direction::Upload && !connection.allow_upload)
            || (direction == Direction::Download && !connection.allow_download)
            || (direction == Direction::BothWays
                && (!connection.allow_upload || !connection.allow_download))
        {
            return Err(RunRequestError::DirectionNotAllowed);
        }
        Ok(Self {
            connection,
            provider,
            direction,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunRequestError {
    ConnectionNotFound,
    ProviderNotFound,
    BothWaysUnsupported,
    DirectionNotAllowed,
}

impl fmt::Display for RunRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConnectionNotFound => formatter.write_str("The connection no longer exists."),
            Self::ProviderNotFound => {
                formatter.write_str("The connection's provider no longer exists.")
            }
            Self::BothWaysUnsupported => {
                formatter.write_str("Both ways is available only for add-only connections.")
            }
            Self::DirectionNotAllowed => {
                formatter.write_str("This direction is not allowed by the connection settings.")
            }
        }
    }
}

impl Error for RunRequestError {}

#[cfg(test)]
mod tests;
