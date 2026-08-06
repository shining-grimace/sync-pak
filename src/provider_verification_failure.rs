use crate::{
    provider_capabilities::ProviderError, provider_network_access::NetworkAccessFailure,
    provider_verification_panic::VerificationPanic,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum VerificationFailure {
    Authentication,
    BucketNotVisible,
    ClockSkew,
    InvalidSettings,
    NetworkInspection,
    NetworkPermission,
    PermissionDenied,
    RuntimeInitialization,
    Unavailable,
    UnexpectedResponse,
    Unsupported,
    WorkerPanicked(VerificationPanic),
    WorkerStart,
    WorkerStopped,
}

impl From<ProviderError> for VerificationFailure {
    fn from(error: ProviderError) -> Self {
        match error {
            ProviderError::Authentication => Self::Authentication,
            ProviderError::ClockSkew => Self::ClockSkew,
            ProviderError::InvalidRequest => Self::InvalidSettings,
            ProviderError::NotFound => Self::BucketNotVisible,
            ProviderError::PermissionDenied => Self::PermissionDenied,
            ProviderError::Unavailable => Self::Unavailable,
            ProviderError::Unsupported => Self::Unsupported,
            ProviderError::Unexpected => Self::UnexpectedResponse,
        }
    }
}

impl From<NetworkAccessFailure> for VerificationFailure {
    fn from(error: NetworkAccessFailure) -> Self {
        match error {
            NetworkAccessFailure::PermissionMissing => Self::NetworkPermission,
            NetworkAccessFailure::InspectionUnavailable => Self::NetworkInspection,
        }
    }
}

impl VerificationFailure {
    pub(crate) fn diagnostic(&self) -> &str {
        match self {
            Self::Authentication => "provider rejected credentials",
            Self::BucketNotVisible => "configured bucket is not visible",
            Self::ClockSkew => "device clock differs from provider",
            Self::InvalidSettings => "provider settings produced an invalid request",
            Self::NetworkInspection => "Android network permission check failed",
            Self::NetworkPermission => "Android INTERNET permission is missing",
            Self::PermissionDenied => "provider denied bucket listing",
            Self::RuntimeInitialization => "provider runtime initialization failed",
            Self::Unavailable => "provider could not be reached",
            Self::UnexpectedResponse => "provider returned an unclassified response",
            Self::Unsupported => "provider does not support the verification request",
            Self::WorkerPanicked(failure) => failure.technical_details(),
            Self::WorkerStart => "provider verification worker could not be started",
            Self::WorkerStopped => "provider verification worker stopped without a result",
        }
    }

    pub(crate) fn message(&self) -> &'static str {
        match self {
            Self::Authentication => {
                "The provider rejected these credentials. Check the access key, secret, and session token."
            }
            Self::BucketNotVisible => {
                "The configured default bucket is not visible to these credentials. Choose another bucket or update its access."
            }
            Self::ClockSkew => {
                "Your device clock differs too much from this provider. Enable automatic date and time before verifying again."
            }
            Self::InvalidSettings => {
                "SyncPak could not build a valid provider request. Check the account ID, region, endpoint, and bucket name."
            }
            Self::NetworkInspection => {
                "SyncPak could not check whether this Android build has network access. Open Diagnostics and report this error."
            }
            Self::NetworkPermission => {
                "This Android build is not allowed to access the internet, so SyncPak cannot contact the provider. Open Diagnostics for details."
            }
            Self::PermissionDenied => {
                "These credentials cannot list buckets. Enter a default bucket manually if the provider grants access only to that bucket."
            }
            Self::RuntimeInitialization => {
                "SyncPak could not initialise the provider verification runtime on this device. Open Diagnostics and report this error."
            }
            Self::Unavailable => {
                "SyncPak could not reach this provider. Check the device network connection and the provider endpoint."
            }
            Self::UnexpectedResponse => {
                "The provider returned a response SyncPak could not classify. Open Diagnostics for details."
            }
            Self::Unsupported => {
                "This provider does not support the request SyncPak uses for verification. Check the provider type and S3-compatible endpoint."
            }
            Self::WorkerPanicked(failure) => failure.message(),
            Self::WorkerStart => {
                "SyncPak could not start provider verification on this device. Open Diagnostics and report this error."
            }
            Self::WorkerStopped => {
                "Provider verification stopped before returning a result. Open Diagnostics and report this error."
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VerificationFailure;
    use crate::{
        provider_capabilities::ProviderError, provider_network_access::NetworkAccessFailure,
    };

    #[test]
    fn maps_every_provider_error_to_a_specific_category() {
        assert_eq!(
            VerificationFailure::from(ProviderError::InvalidRequest),
            VerificationFailure::InvalidSettings
        );
        assert_eq!(
            VerificationFailure::from(ProviderError::Unsupported),
            VerificationFailure::Unsupported
        );
        assert_eq!(
            VerificationFailure::from(ProviderError::Unexpected),
            VerificationFailure::UnexpectedResponse
        );
    }

    #[test]
    fn identifies_android_network_permission_failures() {
        assert_eq!(
            VerificationFailure::from(NetworkAccessFailure::PermissionMissing),
            VerificationFailure::NetworkPermission
        );
        assert!(
            VerificationFailure::NetworkPermission
                .message()
                .contains("not allowed to access the internet")
        );
    }
}
