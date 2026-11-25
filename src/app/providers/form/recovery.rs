use std::rc::Rc;

use slint::ComponentHandle;

use crate::{
    AppWindow, ConfirmationKind,
    app::{diagnostics::SharedDiagnosticLog, providers::save_error::ProviderPersistenceError},
    capabilities::CredentialBackend,
    configuration::ConfigStore,
    platform::credential_discovery::{self, CredentialProvider},
};

pub(super) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    super::recovery_details::configure(window, configuration);
    let weak = window.as_weak();
    window.on_cancel_provider_recovery(move || {
        if let Some(window) = weak.upgrade() {
            window.set_pending_provider_recovery_id(Default::default());
            window.set_confirmation(ConfirmationKind::None);
        }
    });

    let weak = window.as_weak();
    let configuration = Rc::clone(configuration);
    window.on_confirm_provider_recovery(move || {
        let Some(window) = weak.upgrade() else { return };
        let kind = window.get_confirmation();
        let id = window.get_pending_provider_recovery_id();
        let destination_ready = window.get_credential_recovery_destination_ready();
        window.invoke_cancel_provider_recovery();
        match kind {
            ConfirmationKind::RetryProviderAccess => window.invoke_request_provider_edit(id),
            ConfirmationKind::ReplaceProviderCredentials => {
                open(&window, &configuration, Rc::clone(&diagnostics), id);
            }
            ConfirmationKind::SetupProviderCredentialStore if destination_ready => {
                open(&window, &configuration, Rc::clone(&diagnostics), id);
            }
            ConfirmationKind::SetupProviderCredentialStore => window.invoke_request_navigation(3),
            _ => {}
        }
    });
}

fn open(
    window: &AppWindow,
    configuration: &ConfigStore,
    diagnostics: SharedDiagnosticLog,
    id: slint::SharedString,
) {
    let Ok(config) = configuration.load() else {
        window.set_status_message("Provider settings could not be loaded.".into());
        return;
    };
    let Some(provider) = config
        .providers
        .into_iter()
        .find(|p| p.id.as_str() == id.as_str())
    else {
        window.set_status_message("The provider no longer exists.".into());
        return;
    };
    let preferred = config.credential_backend;
    let recovery_id = id.clone();
    super::storage_task::start(
        window,
        move |_| {
            let providers = credential_discovery::discover()
                .map_err(ProviderPersistenceError::ProtectedStore)?;
            Ok(replacement_store(preferred, &providers))
        },
        move |window, result| match result {
            Ok(Some(backend)) => {
                super::state::clear_transient_state(window);
                window.set_provider_form_replacing(true);
                window.set_provider_replacement_store(backend.name().into());
                super::edit::populate(window, provider, String::new());
            }
            Ok(None) => {
                window.set_pending_provider_recovery_id(recovery_id);
                window.set_confirmation(ConfirmationKind::SetupProviderCredentialStore);
            }
            Err(error) => super::credential_task::present_error(window, &diagnostics, &error),
        },
    );
}

pub(super) fn replacement_store(
    preferred: Option<CredentialBackend>,
    providers: &[CredentialProvider],
) -> Option<CredentialBackend> {
    preferred
        .filter(|backend| {
            providers
                .iter()
                .any(|provider| provider.backend == *backend && provider.configured)
        })
        .or_else(|| {
            credential_discovery::preferred(
                providers,
                &std::env::var("XDG_CURRENT_DESKTOP").unwrap_or_default(),
            )
        })
}

pub(super) fn destination(window: &AppWindow) -> Result<Option<CredentialBackend>, &'static str> {
    let store = window.get_provider_replacement_store();
    if !window.get_provider_form_replacing() {
        return Ok(None);
    }
    [
        CredentialBackend::System,
        CredentialBackend::Kde,
        CredentialBackend::KdeLegacy6,
        CredentialBackend::KdeLegacy5,
        CredentialBackend::Gnome,
    ]
    .into_iter()
    .find(|backend| backend.name() == store.as_str())
    .map(Some)
    .ok_or("The replacement credential store is unavailable.")
}
