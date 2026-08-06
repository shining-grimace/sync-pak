use std::path::PathBuf;

use crate::{
    configuration::{ConfigStore, ProviderRepository},
    platform::PlatformCredentialStore,
    provider_verification::ProviderVerification,
    provider_verification_failure::VerificationFailure as ProviderFailure,
};

/// Verifies a saved provider using credentials that remain in protected storage.
pub(crate) fn verify(
    configuration_path: PathBuf,
    provider_id: String,
) -> Result<ProviderVerification, VerificationFailure> {
    let configuration = ConfigStore::at(configuration_path);
    let provider = configuration
        .load()
        .map_err(|_| VerificationFailure::Unexpected)?
        .providers
        .into_iter()
        .find(|provider| provider.id.as_str() == provider_id)
        .ok_or(VerificationFailure::Unexpected)?;
    let store = PlatformCredentialStore::new().map_err(|_| VerificationFailure::Credentials)?;
    let credentials = ProviderRepository::new(&configuration, &store)
        .load_credentials(&provider.id)
        .map_err(|_| VerificationFailure::Credentials)?;
    verify_provider(provider, credentials)
}

#[cfg(feature = "provider-s3")]
fn verify_provider(
    provider: crate::configuration::ProviderConfig,
    credentials: crate::configuration::ProviderCredentials,
) -> Result<ProviderVerification, VerificationFailure> {
    crate::s3_provider_verification_worker::verify(&provider, credentials)
        .map_err(VerificationFailure::Provider)
}

#[cfg(not(feature = "provider-s3"))]
fn verify_provider(
    _: crate::configuration::ProviderConfig,
    _: crate::configuration::ProviderCredentials,
) -> Result<ProviderVerification, VerificationFailure> {
    Err(VerificationFailure::Provider(ProviderFailure::Unavailable))
}

#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
#[derive(Clone, Copy)]
pub(crate) enum VerificationFailure {
    Credentials,
    Provider(ProviderFailure),
    Unexpected,
}

impl VerificationFailure {
    pub(crate) fn diagnostic(self) -> &'static str {
        match self {
            Self::Credentials => "saved credential access failed",
            Self::Provider(failure) => failure.diagnostic(),
            Self::Unexpected => "saved provider verification failed",
        }
    }

    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::Credentials => {
                "SyncPak could not access the saved credentials. Unlock protected storage, then try again."
            }
            Self::Provider(failure) => failure.message(),
            Self::Unexpected => {
                "SyncPak could not load this provider for verification. Open Diagnostics for details."
            }
        }
    }
}

#[cfg(all(test, feature = "provider-s3"))]
mod tests {
    use super::VerificationFailure;
    use crate::provider_capabilities::ProviderError;

    #[test]
    fn classifies_saved_provider_failures_for_safe_recovery() {
        assert!(
            VerificationFailure::Credentials
                .message()
                .contains("protected storage")
        );
        assert_eq!(
            VerificationFailure::Provider(ProviderError::Authentication.into()).diagnostic(),
            "provider rejected credentials"
        );
        assert_eq!(
            VerificationFailure::Provider(ProviderError::NotFound.into()).diagnostic(),
            "configured bucket is not visible"
        );
        assert_eq!(
            VerificationFailure::Provider(ProviderError::PermissionDenied.into()).diagnostic(),
            "provider denied bucket listing"
        );
    }
}
