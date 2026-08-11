#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::{cell::RefCell, collections::HashMap, rc::Rc, sync::mpsc, time::Duration};

use crate::{
    AppWindow,
    app::diagnostics::{self as diagnostics_controller, SharedDiagnosticLog},
    app::providers::bucket_cache::{self as provider_bucket_cache, ProviderBucketCache},
    app::providers::saved_verification::{
        self as saved_provider_verification, VerificationFailure,
    },
    configuration::ConfigStore,
    providers::verification::ProviderVerification,
    providers::verification::failure::VerificationFailure as ProviderFailure,
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
    if let Some(window) = weak.upgrade() {
        window.set_status_message(Default::default());
        window.set_notice_message(Default::default());
    }
    states
        .borrow_mut()
        .insert(provider_id.clone(), VerificationState::Checking);
    crate::app::providers::list_controller::refresh(
        weak,
        &configuration,
        &diagnostics,
        &states,
        &buckets,
    );
    let (sender, receiver) = mpsc::sync_channel(1);
    let configuration_path = configuration.path().to_path_buf();
    let awaiting_id = provider_id.clone();
    let worker_sender = sender.clone();
    let worker = std::thread::Builder::new()
        .name("saved-provider-verification".to_owned())
        .spawn(move || {
            let _ = worker_sender.send(saved_provider_verification::verify(
                configuration_path,
                provider_id,
            ));
        });
    if worker.is_err() {
        let _ = sender.send(Err(VerificationFailure::Provider(
            ProviderFailure::WorkerStart,
        )));
    }
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
                match configuration.record_provider_verification(&provider_id) {
                    Ok(true) => window.set_notice_message(
                        format!(
                            "Provider verified. {} buckets available.",
                            verification.buckets.len()
                        )
                        .into(),
                    ),
                    Ok(false) | Err(_) => diagnostics_controller::present(
                        &window,
                        &diagnostics,
                        "Provider verified but its status could not be saved",
                        "provider verification status save failed",
                        "The provider is verified for this session, but SyncPak could not save that status.",
                    ),
                }
                crate::app::providers::list_controller::refresh(
                    &weak,
                    &configuration,
                    &diagnostics,
                    &states,
                    &buckets,
                );
            }
            Ok(Err(failure)) => {
                states.borrow_mut().remove(&provider_id);
                diagnostics_controller::present_with_safe_details(
                    &window,
                    &diagnostics,
                    "Provider could not be verified",
                    failure.diagnostic().into_owned(),
                    failure.message(),
                );
                crate::app::providers::list_controller::refresh(
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
                    "Provider verification stopped before returning a result. Open Diagnostics and report this error.",
                );
                crate::app::providers::list_controller::refresh(
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
    previously_verified: bool,
) -> &'static str {
    match states.borrow().get(provider_id) {
        Some(VerificationState::Checking) => "Checking",
        None if provider_bucket_cache::buckets(buckets, provider_id).is_some() => {
            "Verified this session"
        }
        None if previously_verified => "Previously verified",
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
mod tests;
