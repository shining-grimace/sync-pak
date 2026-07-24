use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::mpsc, time::Duration};

use crate::{
    AppWindow,
    configuration::ConfigStore,
    diagnostics_controller::{self, SharedDiagnosticLog},
    provider_bucket_cache::{self, ProviderBucketCache},
    provider_verification::ProviderVerification,
    saved_provider_verification::{self, VerificationFailure},
};

pub(crate) type VerificationStates = Rc<RefCell<HashMap<String, VerificationState>>>;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum VerificationState {
    Checking,
}

pub(crate) fn verify(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    states: VerificationStates,
    buckets: ProviderBucketCache,
    provider_id: String,
) {
    if states.borrow().contains_key(&provider_id) {
        return;
    }
    states
        .borrow_mut()
        .insert(provider_id.clone(), VerificationState::Checking);
    crate::provider_list_controller::refresh(weak, &configuration, &diagnostics, &states, &buckets);
    let (sender, receiver) = mpsc::sync_channel(1);
    let configuration_path = configuration.path().to_path_buf();
    let awaiting_id = provider_id.clone();
    std::thread::spawn(move || {
        let _ = sender.send(saved_provider_verification::verify(
            configuration_path,
            provider_id,
        ));
    });
    await_verification(
        weak.clone(),
        configuration,
        diagnostics,
        states,
        buckets,
        awaiting_id,
        receiver,
    );
}

fn await_verification(
    weak: slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    states: VerificationStates,
    buckets: ProviderBucketCache,
    provider_id: String,
    receiver: mpsc::Receiver<Result<ProviderVerification, VerificationFailure>>,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(window) = weak.upgrade() else { return };
        if window.get_page() != 1 {
            states.borrow_mut().remove(&provider_id);
            return;
        }
        match receiver.try_recv() {
            Ok(Ok(verification)) => {
                states.borrow_mut().remove(&provider_id);
                provider_bucket_cache::record(&buckets, &provider_id, verification.buckets.clone());
                window.set_notice_message(
                    format!(
                        "Provider verified. {} buckets available.",
                        verification.buckets.len()
                    )
                    .into(),
                );
                crate::provider_list_controller::refresh(
                    &weak,
                    &configuration,
                    &diagnostics,
                    &states,
                    &buckets,
                );
            }
            Ok(Err(failure)) => {
                states.borrow_mut().remove(&provider_id);
                diagnostics_controller::present(
                    &window,
                    &diagnostics,
                    "Provider could not be verified",
                    failure.diagnostic(),
                    failure.message(),
                );
                crate::provider_list_controller::refresh(
                    &weak,
                    &configuration,
                    &diagnostics,
                    &states,
                    &buckets,
                );
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                states.borrow_mut().remove(&provider_id);
                diagnostics_controller::present(
                    &window,
                    &diagnostics,
                    "Provider could not be verified",
                    "saved provider verification worker stopped",
                    "SyncPak could not complete provider verification. Try again.",
                );
                crate::provider_list_controller::refresh(
                    &weak,
                    &configuration,
                    &diagnostics,
                    &states,
                    &buckets,
                );
            }
            Err(mpsc::TryRecvError::Empty) => await_verification(
                weak,
                configuration,
                diagnostics,
                states,
                buckets,
                provider_id,
                receiver,
            ),
        }
    });
}

pub(crate) fn status(
    states: &VerificationStates,
    buckets: &ProviderBucketCache,
    provider_id: &str,
) -> &'static str {
    match states.borrow().get(provider_id) {
        Some(VerificationState::Checking) => "Checking",
        None if provider_bucket_cache::buckets(buckets, provider_id).is_some() => {
            "Verified this session"
        }
        None => "Not verified",
    }
}

pub(crate) fn is_checking(states: &VerificationStates, provider_id: &str) -> bool {
    matches!(
        states.borrow().get(provider_id),
        Some(VerificationState::Checking)
    )
}

#[cfg(test)]
mod tests {
    use super::{VerificationStates, status};
    use crate::provider_bucket_cache::{ProviderBucketCache, record, remove};

    #[test]
    fn reports_a_cached_listing_as_verified_until_it_is_invalidated() {
        let states: VerificationStates = Default::default();
        let buckets: ProviderBucketCache = Default::default();

        record(&buckets, "provider", Vec::new());
        assert_eq!(
            status(&states, &buckets, "provider"),
            "Verified this session"
        );

        remove(&buckets, "provider");
        assert_eq!(status(&states, &buckets, "provider"), "Not verified");
    }
}
