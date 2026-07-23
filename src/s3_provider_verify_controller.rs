use std::{sync::mpsc, time::Duration};

use slint::{ModelRc, VecModel};

use crate::{
    AppWindow,
    configuration::{ProviderConfig, ProviderCredentials},
    diagnostics_controller::{self, SharedDiagnosticLog},
    provider_capabilities::ProviderError,
    provider_verification::ProviderVerification,
    s3_provider_verification::verify_s3_provider,
};

pub(crate) fn start(
    weak: slint::Weak<AppWindow>,
    provider: ProviderConfig,
    credentials: ProviderCredentials,
    diagnostics: SharedDiagnosticLog,
) {
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build();
        let result = runtime
            .map_err(|_| VerificationFailure::Unexpected)
            .and_then(|runtime| {
                runtime
                    .block_on(verify_s3_provider(&provider, credentials))
                    .map_err(VerificationFailure::from)
            });
        let _ = sender.send(result);
    });
    poll(weak, receiver, diagnostics);
}

fn poll(
    weak: slint::Weak<AppWindow>,
    receiver: mpsc::Receiver<Result<ProviderVerification, VerificationFailure>>,
    diagnostics: SharedDiagnosticLog,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(window) = weak.upgrade() else { return };
        if window.get_page() != 2 || !window.get_provider_verifying() {
            return;
        }
        match receiver.try_recv() {
            Ok(Ok(verification)) => {
                window.set_provider_verifying(false);
                window.set_provider_bucket_list_empty(verification.buckets.is_empty());
                window.set_provider_verified_buckets(ModelRc::new(std::rc::Rc::new(
                    VecModel::from_iter(verification.buckets.iter().cloned().map(Into::into)),
                )));
                window.set_status_message(
                    format!(
                        "Provider verified. {} buckets available.",
                        verification.buckets.len()
                    )
                    .into(),
                );
            }
            Ok(Err(failure)) => {
                window.set_provider_verifying(false);
                diagnostics_controller::present(
                    &window,
                    &diagnostics,
                    "Provider could not be verified",
                    failure.diagnostic(),
                    failure.message(),
                );
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                window.set_provider_verifying(false);
                diagnostics_controller::present(
                    &window,
                    &diagnostics,
                    "Provider could not be verified",
                    "provider verification worker stopped",
                    "SyncPak could not complete provider verification. Try again.",
                );
            }
            Err(mpsc::TryRecvError::Empty) => poll(weak, receiver, diagnostics),
        }
    });
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum VerificationFailure {
    Authentication,
    BucketNotVisible,
    ClockSkew,
    PermissionDenied,
    Unavailable,
    Unexpected,
}

impl From<ProviderError> for VerificationFailure {
    fn from(error: ProviderError) -> Self {
        match error {
            ProviderError::Authentication => Self::Authentication,
            ProviderError::NotFound => Self::BucketNotVisible,
            ProviderError::ClockSkew => Self::ClockSkew,
            ProviderError::PermissionDenied => Self::PermissionDenied,
            ProviderError::Unavailable => Self::Unavailable,
            _ => Self::Unexpected,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::VerificationFailure;
    use crate::provider_capabilities::ProviderError;

    #[test]
    fn maps_provider_errors_to_safe_recovery_categories() {
        assert_eq!(
            VerificationFailure::from(ProviderError::Authentication),
            VerificationFailure::Authentication
        );
        assert_eq!(
            VerificationFailure::from(ProviderError::NotFound),
            VerificationFailure::BucketNotVisible
        );
        assert_eq!(
            VerificationFailure::from(ProviderError::PermissionDenied),
            VerificationFailure::PermissionDenied
        );
        assert_eq!(
            VerificationFailure::from(ProviderError::Unavailable),
            VerificationFailure::Unavailable
        );
        assert_eq!(
            VerificationFailure::from(ProviderError::ClockSkew),
            VerificationFailure::ClockSkew
        );
    }

    #[test]
    fn recovery_messages_remain_specific_without_exposing_credentials() {
        let authentication = VerificationFailure::Authentication.message();
        let inaccessible_bucket = VerificationFailure::BucketNotVisible.message();
        let clock_skew = VerificationFailure::ClockSkew.message();
        let denied = VerificationFailure::PermissionDenied.message();

        assert!(authentication.contains("access key, secret, and session token"));
        assert!(inaccessible_bucket.contains("not visible"));
        assert!(clock_skew.contains("automatic date and time"));
        assert!(denied.contains("cannot list buckets"));
        assert!(!authentication.contains("AKIA"));
        assert!(!inaccessible_bucket.contains("AKIA"));
        assert!(!denied.contains("AKIA"));
    }
}

impl VerificationFailure {
    fn diagnostic(self) -> &'static str {
        match self {
            Self::Authentication => "provider rejected credentials",
            Self::BucketNotVisible => "configured bucket is not visible",
            Self::ClockSkew => "device clock differs from provider",
            Self::PermissionDenied => "provider denied bucket listing",
            Self::Unavailable => "provider could not be reached",
            Self::Unexpected => "provider verification failed",
        }
    }

    fn message(self) -> &'static str {
        match self {
            Self::Authentication => {
                "The provider rejected these credentials. Check the access key, secret, and session token."
            }
            Self::BucketNotVisible => {
                "The configured default bucket is not visible to these credentials. Choose another bucket or update its access."
            }
            Self::ClockSkew => {
                "Your device clock differs too much from this provider. Enable automatic date and time, then try again."
            }
            Self::PermissionDenied => {
                "These credentials cannot list buckets. Enter a default bucket manually if the provider grants access only to that bucket."
            }
            Self::Unavailable => {
                "SyncPak could not reach this provider. Check your network connection and endpoint, then try again."
            }
            Self::Unexpected => {
                "SyncPak could not verify these settings. Check them and try again."
            }
        }
    }
}
