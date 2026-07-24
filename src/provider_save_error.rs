use crate::{capabilities::CapabilityError, configuration::CredentialError};

/// Safe, actionable presentation categories for provider-save failures.
pub(crate) enum ProviderSaveError {
    ProtectedStore(CapabilityError),
    Other,
}

impl From<CredentialError> for ProviderSaveError {
    fn from(error: CredentialError) -> Self {
        match error {
            CredentialError::ProtectedStore(error) => Self::ProtectedStore(error),
            _ => Self::Other,
        }
    }
}

impl ProviderSaveError {
    pub(crate) fn presentation(&self) -> (&'static str, &'static str, &'static str) {
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
}

#[cfg(test)]
mod tests {
    use super::ProviderSaveError;
    use crate::capabilities::CapabilityError;

    #[test]
    fn protected_storage_messages_are_actionable_and_secret_free() {
        let (_, technical, message) =
            ProviderSaveError::ProtectedStore(CapabilityError::Unavailable).presentation();

        assert!(technical.contains("credential storage"));
        assert!(message.contains("Unlock"));
        assert!(!message.contains("key"));
    }
}
