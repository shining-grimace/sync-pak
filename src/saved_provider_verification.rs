use std::path::PathBuf;

use crate::{
    configuration::{ConfigStore, ProviderRepository},
    platform::PlatformCredentialStore,
    provider_verification::ProviderVerification,
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
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| VerificationFailure::Unexpected)?;
    runtime
        .block_on(crate::s3_provider_verification::verify_s3_provider(
            &provider,
            credentials,
        ))
        .map_err(VerificationFailure::from)
}

#[cfg(not(feature = "provider-s3"))]
fn verify_provider(
    _: crate::configuration::ProviderConfig,
    _: crate::configuration::ProviderCredentials,
) -> Result<ProviderVerification, VerificationFailure> {
    Err(VerificationFailure::Unavailable)
}

#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
#[derive(Clone, Copy)]
pub(crate) enum VerificationFailure {
    Credentials,
    Authentication,
    BucketNotVisible,
    PermissionDenied,
    Unavailable,
    Unexpected,
}

#[cfg(feature = "provider-s3")]
impl From<crate::provider_capabilities::ProviderError> for VerificationFailure {
    fn from(error: crate::provider_capabilities::ProviderError) -> Self {
        match error {
            crate::provider_capabilities::ProviderError::Authentication => Self::Authentication,
            crate::provider_capabilities::ProviderError::NotFound => Self::BucketNotVisible,
            crate::provider_capabilities::ProviderError::PermissionDenied => Self::PermissionDenied,
            crate::provider_capabilities::ProviderError::Unavailable => Self::Unavailable,
            _ => Self::Unexpected,
        }
    }
}

impl VerificationFailure {
    pub(crate) fn diagnostic(self) -> &'static str {
        match self {
            Self::Credentials => "saved credential access failed",
            Self::Authentication => "provider rejected saved credentials",
            Self::BucketNotVisible => "configured bucket is not visible",
            Self::PermissionDenied => "provider denied bucket listing",
            Self::Unavailable => "provider could not be reached",
            Self::Unexpected => "saved provider verification failed",
        }
    }

    pub(crate) fn message(self) -> &'static str {
        match self {
            Self::Credentials => {
                "SyncPak could not access the saved credentials. Unlock protected storage, then try again."
            }
            Self::Authentication => {
                "The provider rejected the saved credentials. Update them, then try again."
            }
            Self::BucketNotVisible => {
                "The configured default bucket is not visible to these credentials. Update the bucket or its access, then try again."
            }
            Self::PermissionDenied => {
                "These credentials cannot list buckets. Enter a default bucket manually if the provider grants access only to that bucket."
            }
            Self::Unavailable => {
                "SyncPak could not reach this provider. Check your network connection and try again."
            }
            Self::Unexpected => "SyncPak could not verify this provider. Try again.",
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
            VerificationFailure::from(ProviderError::Authentication).diagnostic(),
            "provider rejected saved credentials"
        );
        assert_eq!(
            VerificationFailure::from(ProviderError::NotFound).diagnostic(),
            "configured bucket is not visible"
        );
        assert_eq!(
            VerificationFailure::from(ProviderError::PermissionDenied).diagnostic(),
            "provider denied bucket listing"
        );
    }
}
