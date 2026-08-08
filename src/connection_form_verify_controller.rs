use std::{rc::Rc, sync::mpsc, time::Duration};

use slint::{ComponentHandle, SharedString};

use crate::{
    AppWindow,
    configuration::ConfigStore,
    connection_form_data::{begin_verification, draft, is_current_verification},
    connection_verification_worker::{self, VerificationFailure},
    diagnostics_controller::{self, SharedDiagnosticLog},
    form_validation,
};

pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let weak = window.as_weak();
    let verify_configuration = Rc::clone(configuration);
    let verify_diagnostics = Rc::clone(&diagnostics);
    window.on_verify_connection(move || {
        request(
            &weak,
            Rc::clone(&verify_configuration),
            Rc::clone(&verify_diagnostics),
            false,
        )
    });

    let weak = window.as_weak();
    let save_configuration = Rc::clone(configuration);
    window.on_save_and_verify_connection(move || {
        request(
            &weak,
            Rc::clone(&save_configuration),
            Rc::clone(&diagnostics),
            true,
        )
    });
}

fn request(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    save_after_verification: bool,
) {
    let Some(window) = weak.upgrade() else { return };
    window.set_connection_save_after_verification(save_after_verification);
    window.set_status_message(SharedString::default());
    window.set_notice_message(SharedString::default());
    let name = window.get_connection_form_name();
    let provider = window.get_connection_form_provider();
    let bucket = window.get_connection_form_bucket();
    let remote = window.get_connection_form_remote();
    let local = window.get_connection_form_local();
    let mode = window.get_connection_form_mode();
    let retention = window.get_connection_form_retention();
    if let Err(error) =
        form_validation::connection(&name, provider, &bucket, &local, mode, &retention)
    {
        window.set_connection_save_after_verification(false);
        window.set_status_message(error.into());
        return;
    }
    let connection = match draft(
        &configuration,
        name,
        provider,
        bucket,
        remote,
        local,
        mode,
        retention,
    ) {
        Ok(connection) => connection,
        Err(_) => {
            window.set_connection_save_after_verification(false);
            window.set_status_message("The connection settings are no longer available.".into());
            return;
        }
    };
    let generation = begin_verification(&window);
    let (sender, receiver) = mpsc::sync_channel(1);
    let configuration_path = configuration.path().to_path_buf();
    std::thread::spawn(move || {
        let _ = sender.send(connection_verification_worker::verify(
            configuration_path,
            connection,
        ));
    });
    poll(weak.clone(), receiver, diagnostics, generation);
}

fn poll(
    weak: slint::Weak<AppWindow>,
    receiver: mpsc::Receiver<Result<(), VerificationFailure>>,
    diagnostics: SharedDiagnosticLog,
    generation: i32,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(window) = weak.upgrade() else { return };
        if window.get_page() != 5
            || !window.get_connection_verifying()
            || !is_current_verification(generation, window.get_connection_verification_generation())
        {
            return;
        }
        match receiver.try_recv() {
            Ok(Ok(())) => verified(&window),
            Ok(Err(failure)) => failed(&window, &diagnostics, failure),
            Err(mpsc::TryRecvError::Disconnected) => {
                failed(&window, &diagnostics, VerificationFailure::Unexpected)
            }
            Err(mpsc::TryRecvError::Empty) => poll(weak, receiver, diagnostics, generation),
        }
    });
}

fn verified(window: &AppWindow) {
    window.set_connection_verifying(false);
    window.set_notice_message("Connection verified. Both paths are accessible.".into());
    if window.get_connection_save_after_verification() {
        window.invoke_save_connection(
            window.get_connection_form_name(),
            window.get_connection_form_provider(),
            window.get_connection_form_bucket(),
            window.get_connection_form_remote(),
            window.get_connection_form_local(),
            window.get_connection_form_mode(),
            window.get_connection_form_retention(),
        );
    }
}

fn failed(window: &AppWindow, diagnostics: &SharedDiagnosticLog, failure: VerificationFailure) {
    window.set_connection_verifying(false);
    window.set_connection_save_after_verification(false);
    diagnostics_controller::present_with_safe_details(
        window,
        diagnostics,
        "Connection could not be verified",
        failure.diagnostic().into_owned(),
        failure.message(),
    );
}
