use crate::{capabilities::CapabilityError, configuration::CredentialError};

/// Safe, actionable presentation categories for provider credential persistence failures.
pub(crate) enum ProviderPersistenceError {
    ProtectedStore(CapabilityError),
    Other,
}

impl From<CredentialError> for ProviderPersistenceError {
    fn from(error: CredentialError) -> Self {
        match error {
            CredentialError::ProtectedStore(error) => Self::ProtectedStore(error),
            _ => Self::Other,
        }
    }
}

impl ProviderPersistenceError {
    pub(crate) fn save_presentation(&self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::ProtectedStore(CapabilityError::Busy) => (
                "Protected credential storage is busy",
                "protected credential storage busy",
                "Another protected-storage request is in progress. Wait a moment, then try saving again.",
            ),
            Self::ProtectedStore(_) => (
                "Protected credential storage is unavailable",
                "protected credential storage unavailable",
                "SyncPak could not securely save these credentials. Unlock your device's credential store, then try again.",
            ),
            Self::Other => (
                "Provider settings could not be saved",
                "provider save failed",
                "SyncPak could not save this provider. Check its settings and protected storage, then try again.",
            ),
        }
    }

    pub(crate) fn delete_presentation(&self) -> (&'static str, &'static str, &'static str) {
        match self {
            Self::ProtectedStore(CapabilityError::Busy) => (
                "Protected credential storage is busy",
                "protected credential storage busy during provider deletion",
                "Another protected-storage request is in progress. Wait a moment, then try deleting this provider again.",
            ),
            Self::ProtectedStore(_) => (
                "Protected credential storage is unavailable",
                "protected credential storage unavailable during provider deletion",
                "SyncPak could not securely remove this provider's credentials. Unlock your device's credential store, then try again.",
            ),
            Self::Other => (
                "Provider could not be deleted",
                "provider deletion failed",
                "SyncPak could not delete this provider. Check protected storage and try again.",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::ProviderPersistenceError;
    use crate::capabilities::CapabilityError;

    #[test]
    fn protected_storage_messages_are_actionable_and_secret_free() {
        let (_, technical, message) =
            ProviderPersistenceError::ProtectedStore(CapabilityError::Unavailable)
                .save_presentation();

        assert!(technical.contains("credential storage"));
        assert!(message.contains("Unlock"));
        assert!(!message.contains("key"));
    }
}
