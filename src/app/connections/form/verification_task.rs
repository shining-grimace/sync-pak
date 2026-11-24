use std::{sync::mpsc, time::Duration};

use slint::ComponentHandle;

use crate::{
    AppWindow, ConfirmationKind, app::connections::verification_worker::VerificationFailure,
};

use super::data::{begin_verification, is_current_verification};

pub(super) fn start(
    window: &AppWindow,
    work: impl FnOnce() -> Result<(), VerificationFailure> + Send + 'static,
    complete: impl FnOnce(&AppWindow, i32, Result<(), VerificationFailure>) + 'static,
) {
    let generation = begin_verification(window);
    let (sender, receiver) = mpsc::sync_channel(1);
    let thread = std::thread::Builder::new()
        .name("connection-path-check".into())
        .spawn(move || {
            let _ = sender.send(work());
        });
    if thread.is_err() {
        window.set_connection_verifying(false);
        complete(window, generation, Err(VerificationFailure::Unexpected));
        return;
    }
    poll(window.as_weak(), receiver, generation, complete);
}

fn poll(
    weak: slint::Weak<AppWindow>,
    receiver: mpsc::Receiver<Result<(), VerificationFailure>>,
    generation: i32,
    complete: impl FnOnce(&AppWindow, i32, Result<(), VerificationFailure>) + 'static,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(window) = weak.upgrade() else { return };
        if window.get_page() != 5
            || !window.get_connection_verifying()
            || !is_current_verification(generation, window.get_connection_verification_generation())
        {
            return;
        }
        if !matches!(
            window.get_confirmation(),
            ConfirmationKind::None | ConfirmationKind::CreateRemoteFolder
        ) {
            poll(weak, receiver, generation, complete);
            return;
        }
        let result = match receiver.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Disconnected) => Err(VerificationFailure::Unexpected),
            Err(mpsc::TryRecvError::Empty) => {
                poll(weak, receiver, generation, complete);
                return;
            }
        };
        window.set_connection_verifying(false);
        complete(&window, generation, result);
    });
}
