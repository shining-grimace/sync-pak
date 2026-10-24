//! Runs protected-storage work outside the event loop. Completion stays on the UI thread.
use std::{
    sync::mpsc,
    time::{Duration, Instant},
};

use slint::ComponentHandle;

use crate::{
    AppWindow,
    app::providers::save_error::ProviderPersistenceError,
    capabilities::{CredentialInteraction, CredentialPhase},
};

pub(crate) fn start<T: Send + 'static>(
    window: &AppWindow,
    work: impl FnOnce(CredentialInteraction) -> Result<T, ProviderPersistenceError> + Send + 'static,
    complete: impl FnOnce(&AppWindow, Result<T, ProviderPersistenceError>) + 'static,
) {
    if window.get_provider_storage_busy() {
        return;
    }
    window.set_provider_storage_busy(true);
    window.set_provider_storage_waiting(false);
    window.set_status_message(Default::default());
    let interaction = CredentialInteraction::default();
    let cancellation = interaction.clone();
    window.on_cancel_provider_storage(move || {
        cancellation.cancel();
    });
    window.set_provider_storage_can_cancel(interaction.can_cancel());
    let worker_interaction = interaction.clone();
    let receiver = match spawn(move || work(worker_interaction)) {
        Ok(receiver) => receiver,
        Err(_) => {
            window.set_provider_storage_busy(false);
            window.set_provider_storage_can_cancel(false);
            window.on_cancel_provider_storage(|| {});
            complete(window, Err(ProviderPersistenceError::Other));
            return;
        }
    };
    poll(
        window.as_weak(),
        receiver,
        interaction,
        Instant::now(),
        complete,
    );
}

fn spawn<T: Send + 'static>(
    work: impl FnOnce() -> Result<T, ProviderPersistenceError> + Send + 'static,
) -> std::io::Result<mpsc::Receiver<Result<T, ProviderPersistenceError>>> {
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::Builder::new()
        .name("provider-storage".into())
        .spawn(move || {
            let _ = sender.send(work());
        })?;
    Ok(receiver)
}

fn poll<T: 'static>(
    weak: slint::Weak<AppWindow>,
    receiver: mpsc::Receiver<Result<T, ProviderPersistenceError>>,
    interaction: CredentialInteraction,
    started: Instant,
    complete: impl FnOnce(&AppWindow, Result<T, ProviderPersistenceError>) + 'static,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(window) = weak.upgrade() else {
            interaction.cancel();
            return;
        };
        let mut result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Disconnected) => Err(ProviderPersistenceError::Other),
            Err(mpsc::TryRecvError::Empty) => {
                let phase = interaction.phase();
                window.set_provider_storage_message(match phase {
                    CredentialPhase::Checking => "Checking this device's password storage…",
                    CredentialPhase::Unlocking => "Your password storage is locked. Enter its password in the desktop unlock dialog. If no dialog appears, cancel and check your desktop's password manager.",
                    CredentialPhase::Writing => "Finishing the credential request…",
                    CredentialPhase::Cancelled => "Cancelling credential access…",
                }.into());
                window.set_provider_storage_can_cancel(interaction.can_cancel());
                window.set_provider_storage_waiting(
                    phase == CredentialPhase::Unlocking
                        || started.elapsed() >= Duration::from_millis(500),
                );
                poll(weak, receiver, interaction, started, complete);
                return;
            }
        };
        // A quick read can finish before the UI processes Cancel. It still must not
        // start verification or another save stage after cancellation was accepted.
        if interaction.phase() == CredentialPhase::Cancelled {
            result = Err(ProviderPersistenceError::ProtectedStore(
                crate::capabilities::CapabilityError::Cancelled,
            ));
        }
        window.set_provider_storage_busy(false);
        window.set_provider_storage_waiting(false);
        window.set_provider_storage_can_cancel(false);
        window.on_cancel_provider_storage(|| {});
        complete(&window, result);
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waiting_for_storage_does_not_block_the_caller() {
        let (release, wait) = mpsc::sync_channel(1);
        let caller = std::thread::current().id();
        let results = spawn(move || {
            assert_ne!(std::thread::current().id(), caller);
            wait.recv().unwrap();
            Ok(42)
        })
        .unwrap();
        assert!(matches!(results.try_recv(), Err(mpsc::TryRecvError::Empty)));
        release.send(()).unwrap();
        assert!(matches!(
            results.recv_timeout(Duration::from_secs(2)),
            Ok(Ok(42))
        ));
    }
}
