use crate::{
    capabilities::CapabilityError,
    configuration::{ConfigStore, ConnectionError, ConnectionId, ConnectionRepository},
};

/// Cancels an active operation and removes waiting operations for one connection.
///
/// Configuration deletion uses this narrow capability before removing the connection
/// or provider credentials that an operation may still need.
pub trait ConnectionOperationCanceller {
    fn cancel_for_connection(&self, connection_id: &str) -> Result<usize, CapabilityError>;
}

/// Cancels all operations associated with a provider's dependent connections.
pub fn cancel_for_connections<C: ConnectionOperationCanceller>(
    canceller: &C,
    connection_ids: impl IntoIterator<Item = impl AsRef<str>>,
) -> Result<usize, CapabilityError> {
    connection_ids
        .into_iter()
        .try_fold(0, |removed, connection_id| {
            canceller
                .cancel_for_connection(connection_id.as_ref())
                .map(|count| removed + count)
        })
}

/// Cancels a connection's work before deleting its configuration.
pub fn delete_connection<C: ConnectionOperationCanceller>(
    configuration: &ConfigStore,
    canceller: &C,
    connection_id: &ConnectionId,
) -> Result<crate::configuration::ConnectionConfig, ConnectionDeletionError> {
    canceller
        .cancel_for_connection(connection_id.as_str())
        .map_err(ConnectionDeletionError::Cancellation)?;
    ConnectionRepository::new(configuration)
        .delete(connection_id)
        .map_err(ConnectionDeletionError::Configuration)
}

#[derive(Debug)]
pub enum ConnectionDeletionError {
    Cancellation(CapabilityError),
    Configuration(ConnectionError),
}

impl std::fmt::Display for ConnectionDeletionError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Cancellation(error) => {
                write!(formatter, "could not cancel connection work: {error}")
            }
            Self::Configuration(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for ConnectionDeletionError {}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;

    use crate::{
        capabilities::CapabilityError,
        configuration::{ConfigStore, ConnectionId},
    };

    use super::{ConnectionOperationCanceller, cancel_for_connections, delete_connection};

    #[derive(Default)]
    struct Canceller(RefCell<Vec<String>>);

    impl ConnectionOperationCanceller for Canceller {
        fn cancel_for_connection(&self, connection_id: &str) -> Result<usize, CapabilityError> {
            self.0.borrow_mut().push(connection_id.into());
            Ok(1)
        }
    }

    #[test]
    fn cancels_every_provider_dependent_connection() {
        let canceller = Canceller::default();

        assert_eq!(cancel_for_connections(&canceller, ["one", "two"]), Ok(2));
        assert_eq!(canceller.0.into_inner(), ["one", "two"]);
    }

    #[test]
    fn cancellation_happens_before_connection_deletion_is_attempted() {
        let temporary =
            std::env::temp_dir().join(format!("sync-pak-delete-{}", uuid::Uuid::new_v4()));
        let configuration = ConfigStore::at(temporary.join("config.json"));
        let canceller = Canceller::default();
        let connection_id = ConnectionId::new();

        assert!(delete_connection(&configuration, &canceller, &connection_id).is_err());
        assert_eq!(canceller.0.into_inner(), [connection_id.as_str()]);
    }
}
