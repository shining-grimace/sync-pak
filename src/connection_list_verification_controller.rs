use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    rc::Rc,
    sync::mpsc,
    time::Duration,
};

use crate::{
    AppWindow,
    configuration::ConfigStore,
    connection_verification_worker::{self, VerificationFailure},
    diagnostics_controller::{self, SharedDiagnosticLog},
};

pub(crate) type VerificationStates = Rc<RefCell<HashMap<String, VerificationState>>>;
pub(crate) type SessionVerifications = Rc<RefCell<HashSet<String>>>;

#[derive(Clone, Copy, Eq, PartialEq)]
pub(crate) enum VerificationState {
    Checking,
}

pub(crate) fn verify(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    states: VerificationStates,
    sessions: SessionVerifications,
    connection_id: String,
) {
    if states.borrow().contains_key(&connection_id) {
        return;
    }
    if let Some(window) = weak.upgrade() {
        window.set_status_message(Default::default());
        window.set_notice_message(Default::default());
    }
    states
        .borrow_mut()
        .insert(connection_id.clone(), VerificationState::Checking);
    crate::connection_list_controller::refresh(
        weak,
        &configuration,
        &diagnostics,
        &states,
        &sessions,
    );
    let (sender, receiver) = mpsc::sync_channel(1);
    let configuration_path = configuration.path().to_path_buf();
    let awaiting_id = connection_id.clone();
    std::thread::spawn(move || {
        let _ = sender.send(connection_verification_worker::verify_saved(
            configuration_path,
            connection_id,
        ));
    });
    poll(
        weak.clone(),
        configuration,
        diagnostics,
        states,
        sessions,
        awaiting_id,
        receiver,
    );
}

#[allow(clippy::too_many_arguments)]
fn poll(
    weak: slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    states: VerificationStates,
    sessions: SessionVerifications,
    connection_id: String,
    receiver: mpsc::Receiver<Result<(), VerificationFailure>>,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(window) = weak.upgrade() else { return };
        if window.get_page() != 4 {
            states.borrow_mut().remove(&connection_id);
            return;
        }
        match receiver.try_recv() {
            Ok(Ok(())) => {
                states.borrow_mut().remove(&connection_id);
                sessions.borrow_mut().insert(connection_id.clone());
                if !matches!(
                    configuration.record_connection_verification(&connection_id),
                    Ok(true)
                ) {
                    diagnostics_controller::present(
                        &window,
                        &diagnostics,
                        "Connection verified but its status could not be saved",
                        "connection verification status save failed",
                        "The connection is verified for this session, but SyncPak could not save that status.",
                    );
                } else {
                    window.set_notice_message(
                        "Connection verified. Both paths are accessible.".into(),
                    );
                }
                refresh(&weak, &configuration, &diagnostics, &states, &sessions);
            }
            Ok(Err(failure)) => {
                states.borrow_mut().remove(&connection_id);
                diagnostics_controller::present_with_safe_details(
                    &window,
                    &diagnostics,
                    "Connection could not be verified",
                    failure.diagnostic().into_owned(),
                    failure.message(),
                );
                refresh(&weak, &configuration, &diagnostics, &states, &sessions);
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                states.borrow_mut().remove(&connection_id);
                diagnostics_controller::present(
                    &window,
                    &diagnostics,
                    "Connection could not be verified",
                    "connection verification worker stopped",
                    "SyncPak could not complete connection verification. Try again.",
                );
                refresh(&weak, &configuration, &diagnostics, &states, &sessions);
            }
            Err(mpsc::TryRecvError::Empty) => poll(
                weak,
                configuration,
                diagnostics,
                states,
                sessions,
                connection_id,
                receiver,
            ),
        }
    });
}

fn refresh(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
    states: &VerificationStates,
    sessions: &SessionVerifications,
) {
    crate::connection_list_controller::refresh(weak, configuration, diagnostics, states, sessions);
}

pub(crate) fn status(
    states: &VerificationStates,
    sessions: &SessionVerifications,
    connection_id: &str,
    previously_verified: bool,
) -> &'static str {
    if states.borrow().contains_key(connection_id) {
        "Checking"
    } else if sessions.borrow().contains(connection_id) {
        "Verified this session"
    } else if previously_verified {
        "Previously verified"
    } else {
        "Not verified"
    }
}

pub(crate) fn is_checking(states: &VerificationStates, connection_id: &str) -> bool {
    states.borrow().contains_key(connection_id)
}
