use std::{cell::RefCell, rc::Rc};

use slint::{ComponentHandle, SharedString};

use crate::{
    AppWindow, ConfirmationKind,
    app::connections::form::data::is_current_verification,
    app::connections::verification_worker::{
        self as connection_verification_worker, VerificationFailure,
    },
    app::diagnostics::{self as diagnostics_controller, SharedDiagnosticLog},
    configuration::{ConfigStore, ConnectionDraft},
};

use super::{draft::validated_form_draft, save, verification_task};

struct PendingCreation {
    generation: i32,
    connection: ConnectionDraft,
}

type PendingFolder = RefCell<Option<PendingCreation>>;

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
            None,
        )
    });

    let pending = Rc::new(PendingFolder::default());
    let weak = window.as_weak();
    let save_pending = Rc::clone(&pending);
    let save_diagnostics = Rc::clone(&diagnostics);
    let save_configuration = Rc::clone(configuration);
    window.on_save_and_verify_connection(move || {
        request(
            &weak,
            Rc::clone(&save_configuration),
            Rc::clone(&save_diagnostics),
            Some(Rc::clone(&save_pending)),
        )
    });

    let weak = window.as_weak();
    let cancel_pending = Rc::clone(&pending);
    window.on_cancel_create_remote_folder(move || {
        if let Some(window) = weak.upgrade()
            && window.get_confirmation() == ConfirmationKind::CreateRemoteFolder
            && !window.get_connection_verifying()
        {
            cancel_pending.borrow_mut().take();
            window.set_confirmation(ConfirmationKind::None);
        }
    });

    let weak = window.as_weak();
    let configuration = Rc::clone(configuration);
    window.on_confirm_create_remote_folder(move || {
        if let Some(window) = weak.upgrade() {
            create_remote_folder(
                &window,
                Rc::clone(&configuration),
                Rc::clone(&diagnostics),
                &pending,
            );
        }
    });
}

fn request(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    save: Option<Rc<PendingFolder>>,
) {
    let Some(window) = weak.upgrade() else { return };
    if window.get_page() != 5
        || window.get_connection_verifying()
        || window.get_confirmation() != ConfirmationKind::None
    {
        return;
    }
    if let Some(pending) = &save {
        pending.borrow_mut().take();
    }
    window.set_status_message(SharedString::default());
    window.set_notice_message(SharedString::default());
    let Some(connection) = validated_form_draft(&window, &configuration) else {
        return;
    };
    let configuration_path = configuration.path().to_path_buf();
    let verification_draft = connection.clone();
    verification_task::start(
        &window,
        move || connection_verification_worker::verify(configuration_path, verification_draft),
        move |window, generation, result| match (result, save) {
            (Ok(()), Some(_)) => save_verified(window, configuration, &diagnostics, connection),
            (Ok(()), None) => window.set_notice_message(
                "Connection path checked. The local folder and cloud bucket/folder are accessible."
                    .into(),
            ),
            (Err(VerificationFailure::MissingRemotePrefix), Some(pending))
                if window.get_connection_form_id().is_empty() =>
            {
                pending.replace(Some(PendingCreation {
                    generation,
                    connection,
                }));
                window.set_confirmation(ConfirmationKind::CreateRemoteFolder);
            }
            (Err(failure), _) => failed(window, &diagnostics, failure, false),
        },
    );
}

fn create_remote_folder(
    window: &AppWindow,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    pending: &PendingFolder,
) {
    if window.get_page() != 5
        || !window.get_connection_form_id().is_empty()
        || window.get_confirmation() != ConfirmationKind::CreateRemoteFolder
        || window.get_connection_verifying()
    {
        return;
    }
    let Some(PendingCreation {
        generation,
        connection,
    }) = pending.borrow_mut().take()
    else {
        return;
    };
    if !is_current_verification(generation, window.get_connection_verification_generation()) {
        window.set_confirmation(ConfirmationKind::None);
        return;
    }
    let configuration_path = configuration.path().to_path_buf();
    let creation_draft = connection.clone();
    verification_task::start(
        window,
        move || {
            connection_verification_worker::create_remote_folder(configuration_path, creation_draft)
        },
        move |window, _, result| {
            window.set_confirmation(ConfirmationKind::None);
            match result {
                Ok(()) => save_verified(window, configuration, &diagnostics, connection),
                Err(failure) => failed(window, &diagnostics, failure, true),
            }
        },
    );
}

fn save_verified(
    window: &AppWindow,
    configuration: Rc<ConfigStore>,
    diagnostics: &SharedDiagnosticLog,
    mut connection: ConnectionDraft,
) {
    connection.verified = true;
    save::persist(window, configuration, diagnostics, Ok(connection));
}

fn failed(
    window: &AppWindow,
    diagnostics: &SharedDiagnosticLog,
    failure: VerificationFailure,
    creating_folder: bool,
) {
    let (summary, message) = if creating_folder {
        let message = match failure {
            VerificationFailure::PermissionDenied => {
                "The provider denied permission to create the remote folder."
            }
            VerificationFailure::Unexpected => {
                "SyncPak could not create the remote folder. Check the provider settings and try again."
            }
            _ => failure.message(),
        };
        ("Remote folder could not be created", message)
    } else {
        ("Connection could not be verified", failure.message())
    };
    diagnostics_controller::present_with_safe_details(
        window,
        diagnostics,
        summary,
        failure.diagnostic().into_owned(),
        message,
    );
}
