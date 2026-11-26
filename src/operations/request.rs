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
    pub widget_create_only: bool,
}

impl RunRequest {
    pub fn automatic(config: &AppConfig, connection_id: &str) -> Result<Self, RunRequestError> {
        let connection = config
            .connections
            .iter()
            .find(|c| c.id.as_str() == connection_id)
            .ok_or(RunRequestError::ConnectionNotFound)?;
        if connection.show_preview {
            return Err(RunRequestError::PreviewRequired);
        }
        let direction = Self::saved_direction(connection)?;
        if direction == Direction::BothWays && connection.mode == SyncMode::Mirror {
            return Err(RunRequestError::PreviewRequired);
        }
        Self::from_config(config, connection_id, direction)
    }

    #[cfg(any(target_os = "android", test))]
    pub fn widget(
        config: &AppConfig,
        connection_id: &str,
        create_only: bool,
    ) -> Result<Self, RunRequestError> {
        if !create_only {
            return Self::automatic(config, connection_id);
        }
        let connection = config
            .connections
            .iter()
            .find(|c| c.id.as_str() == connection_id)
            .ok_or(RunRequestError::ConnectionNotFound)?;
        if connection.mode == SyncMode::Archive {
            return Err(RunRequestError::PreviewRequired);
        }
        let mut request =
            Self::from_config(config, connection_id, Self::saved_direction(connection)?)?;
        request.widget_create_only = true;
        Ok(request)
    }

    fn saved_direction(connection: &ConnectionConfig) -> Result<Direction, RunRequestError> {
        Ok(match (connection.allow_upload, connection.allow_download) {
            (true, false) => Direction::Upload,
            (false, true) => Direction::Download,
            (true, true) => Direction::BothWays,
            _ => return Err(RunRequestError::DirectionNotAllowed),
        })
    }

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
        if direction == Direction::BothWays && connection.mode == SyncMode::Archive {
            return Err(RunRequestError::BothWaysUnsupported);
        }
        if (direction == Direction::Upload && !connection.allow_upload)
            || (direction == Direction::Download && !connection.allow_download)
            || (direction == Direction::BothWays
                && (!connection.allow_upload || !connection.allow_download))
        {
            return Err(RunRequestError::DirectionNotAllowed);
        }
        if !connection.verified {
            return Err(RunRequestError::NotVerified);
        }
        Ok(Self {
            connection,
            provider,
            direction,
            widget_create_only: false,
        })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RunRequestError {
    ConnectionNotFound,
    ProviderNotFound,
    NotVerified,
    PreviewRequired,
    BothWaysUnsupported,
    DirectionNotAllowed,
}

impl fmt::Display for RunRequestError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::PreviewRequired => {
                formatter.write_str("This connection requires preview before syncing.")
            }
            Self::NotVerified => formatter.write_str("Verify this connection before syncing."),
            Self::ConnectionNotFound => formatter.write_str("The connection no longer exists."),
            Self::ProviderNotFound => {
                formatter.write_str("The connection's provider no longer exists.")
            }
            Self::BothWaysUnsupported => {
                formatter.write_str("Both ways is unavailable for archive connections.")
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
