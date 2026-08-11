#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::{sync::mpsc, time::Duration};

use slint::{ModelRc, VecModel};

use crate::{
    AppWindow,
    app::diagnostics::{self as diagnostics_controller, SharedDiagnosticLog},
    configuration::{ProviderConfig, ProviderCredentials},
    providers::verification::ProviderVerification,
    providers::verification::failure::VerificationFailure,
};

pub(crate) fn start(
    weak: slint::Weak<AppWindow>,
    provider: ProviderConfig,
    credentials: ProviderCredentials,
    diagnostics: SharedDiagnosticLog,
    generation: i32,
) {
    let (sender, receiver) = mpsc::sync_channel(1);
    let worker_sender = sender.clone();
    let worker = std::thread::Builder::new()
        .name("provider-verification".to_owned())
        .spawn(move || {
            let result = crate::providers::s3::verification_worker::verify(&provider, credentials);
            let _ = worker_sender.send(result);
        });
    if worker.is_err() {
        let _ = sender.send(Err(VerificationFailure::WorkerStart));
    }
    poll(weak, receiver, diagnostics, generation);
}

fn poll(
    weak: slint::Weak<AppWindow>,
    receiver: mpsc::Receiver<Result<ProviderVerification, VerificationFailure>>,
    diagnostics: SharedDiagnosticLog,
    generation: i32,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(window) = weak.upgrade() else { return };
        if window.get_page() != 2
            || !window.get_provider_verifying()
            || !crate::app::providers::form::state::is_current_verification(
                generation,
                window.get_provider_verification_generation(),
            )
        {
            return;
        }
        match receiver.try_recv() {
            Ok(Ok(verification)) => {
                window.set_provider_verifying(false);
                let save_after_verification = window.get_provider_save_after_verification();
                window.set_provider_bucket_list_empty(verification.buckets.is_empty());
                window.set_provider_verified_buckets(ModelRc::new(std::rc::Rc::new(
                    VecModel::from_iter(verification.buckets.iter().cloned().map(Into::into)),
                )));
                window.set_notice_message(
                    format!(
                        "Provider verified. {} buckets available.",
                        verification.buckets.len()
                    )
                    .into(),
                );
                if save_after_verification {
                    window.invoke_save_provider(
                        window.get_provider_form_name(),
                        window.get_provider_form_kind(),
                        window.get_provider_form_access_key(),
                        window.get_provider_form_secret_key(),
                    );
                }
            }
            Ok(Err(failure)) => {
                window.set_provider_verifying(false);
                window.set_provider_save_after_verification(false);
                diagnostics_controller::present_with_safe_details(
                    &window,
                    &diagnostics,
                    "Provider could not be verified",
                    failure.diagnostic().into_owned(),
                    failure.message(),
                );
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                window.set_provider_verifying(false);
                window.set_provider_save_after_verification(false);
                diagnostics_controller::present_with_safe_details(
                    &window,
                    &diagnostics,
                    "Provider could not be verified",
                    VerificationFailure::WorkerStopped.diagnostic().into_owned(),
                    VerificationFailure::WorkerStopped.message(),
                );
            }
            Err(mpsc::TryRecvError::Empty) => poll(weak, receiver, diagnostics, generation),
        }
    });
}

#[cfg(test)]
mod tests {
    use crate::providers::capabilities::ProviderError;
    use crate::providers::verification::failure::VerificationFailure;

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
