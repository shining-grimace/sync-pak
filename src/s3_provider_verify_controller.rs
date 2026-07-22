use std::{sync::mpsc, time::Duration};

use crate::{
    AppWindow,
    configuration::{ProviderConfig, ProviderCredentials},
    diagnostics_controller::{self, SharedDiagnosticLog},
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
        let result = runtime.ok().and_then(|runtime| {
            runtime
                .block_on(verify_s3_provider(&provider, credentials))
                .ok()
        });
        let _ = sender.send(result);
    });
    poll(weak, receiver, diagnostics);
}

fn poll(
    weak: slint::Weak<AppWindow>,
    receiver: mpsc::Receiver<Option<ProviderVerification>>,
    diagnostics: SharedDiagnosticLog,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        match receiver.try_recv() {
            Ok(Some(verification)) => {
                if let Some(window) = weak.upgrade() {
                    window.set_provider_verifying(false);
                    window.set_status_message(
                        format!(
                            "Provider verified. {} buckets available.",
                            verification.buckets.len()
                        )
                        .into(),
                    );
                }
            }
            Ok(None) | Err(mpsc::TryRecvError::Disconnected) => {
                if let Some(window) = weak.upgrade() {
                    window.set_provider_verifying(false);
                    diagnostics_controller::present(
                        &window,
                        &diagnostics,
                        "Provider could not be verified",
                        "provider verification failed",
                        "SyncPak could not verify these credentials. Check the settings and try again.",
                    );
                }
            }
            Err(mpsc::TryRecvError::Empty) => poll(weak, receiver, diagnostics),
        }
    });
}
