use std::rc::Rc;

use crate::{
    AppWindow,
    app::{
        diagnostics::{self, SharedDiagnosticLog},
        providers::save_error::ProviderPersistenceError,
    },
    configuration::{ConfigStore, ProviderCredentials},
};

/// Resolve retained secrets before verification or saving without blocking the UI.
pub(super) fn resolve(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostic_log: SharedDiagnosticLog,
    complete: impl FnOnce(&AppWindow, ProviderCredentials) + 'static,
) {
    let edit_id = window.get_provider_form_id();
    let id = if edit_id.is_empty() {
        None
    } else {
        match super::state::provider_id(configuration, edit_id.as_str()) {
            Ok(id) => Some(id),
            Err(error) => {
                window.set_provider_save_after_verification(false);
                window.set_status_message(error.into());
                return;
            }
        }
    };
    let access = window.get_provider_form_access_key().to_string();
    let secret = window.get_provider_form_secret_key().to_string();
    let session = window.get_provider_form_session_token().to_string();
    let configuration = ConfigStore::at(configuration.path().to_owned());
    super::storage_task::start(
        window,
        move |interaction| {
            super::credentials::resolve(
                &configuration,
                id.as_ref(),
                &access,
                &secret,
                &session,
                interaction,
            )
        },
        move |window, result| match result {
            Ok(credentials) => complete(window, credentials),
            Err(error) => {
                window.set_provider_save_after_verification(false);
                window.set_page(2);
                present_error(window, &diagnostic_log, &error);
            }
        },
    );
}

pub(super) fn present_error(
    window: &AppWindow,
    diagnostic_log: &SharedDiagnosticLog,
    error: &ProviderPersistenceError,
) {
    let (summary, technical, message) = error.save_presentation();
    if matches!(
        error,
        ProviderPersistenceError::ProtectedStore(crate::capabilities::CapabilityError::Cancelled)
    ) {
        window.set_notice_message(message.into());
        return;
    }
    diagnostics::present(window, diagnostic_log, summary, technical, message);
}
