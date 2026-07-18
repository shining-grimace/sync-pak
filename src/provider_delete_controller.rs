use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use crate::{
    AppWindow,
    configuration::{ConfigStore, ProviderId, ProviderRepository},
    diagnostics_controller::{self, SharedDiagnosticLog},
    platform::PlatformCredentialStore,
};

pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let weak = window.as_weak();
    let request_config = Rc::clone(configuration);
    window.on_request_provider_delete(move |id| {
        request_delete(&weak, &request_config, id);
    });

    let weak = window.as_weak();
    let confirm_config = Rc::clone(configuration);
    let confirm_diagnostics = Rc::clone(&diagnostics);
    window.on_confirm_provider_delete(move || {
        delete_provider(&weak, &confirm_config, &confirm_diagnostics);
    });

    let weak = window.as_weak();
    let cancel_config = Rc::clone(configuration);
    window.on_cancel_provider_delete(move || {
        crate::app_controller::show_providers(&weak, Rc::clone(&cancel_config));
    });
}

fn request_delete(weak: &slint::Weak<AppWindow>, configuration: &ConfigStore, id: SharedString) {
    let Some(window) = weak.upgrade() else { return };
    match provider_and_dependents(configuration, id.as_str()) {
        Ok((provider, connection_count)) => {
            window.set_pending_provider_id(provider.id.as_str().into());
            window.set_pending_provider_name(provider.name.into());
            window.set_pending_connection_count(connection_count as i32);
            window.set_status_message(SharedString::default());
            window.set_page(6);
        }
        Err(error) => window.set_status_message(error.into()),
    }
}

fn delete_provider(
    weak: &slint::Weak<AppWindow>,
    configuration: &Rc<ConfigStore>,
    diagnostics: &SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    let result =
        provider_id(configuration, window.get_pending_provider_id().as_str()).and_then(|id| {
            PlatformCredentialStore::new()
                .map_err(|error| error.to_string())
                .and_then(|store| {
                    ProviderRepository::new(configuration, &store)
                        .delete(&id)
                        .map_err(|error| error.to_string())
                })
        });
    match result {
        Ok(_) => crate::app_controller::show_providers(weak, Rc::clone(configuration)),
        Err(_) => diagnostics_controller::present(
            &window,
            diagnostics,
            "Provider could not be deleted",
            "provider deletion failed",
            "SyncPak could not delete this provider. Check protected storage and try again.",
        ),
    }
}

fn provider_and_dependents(
    configuration: &ConfigStore,
    id: &str,
) -> Result<(crate::configuration::ProviderConfig, usize), String> {
    let config = configuration.load().map_err(|error| error.to_string())?;
    let provider = config
        .providers
        .iter()
        .find(|provider| provider.id.as_str() == id)
        .cloned()
        .ok_or_else(|| "The provider no longer exists.".to_owned())?;
    let count = config
        .connections
        .iter()
        .filter(|connection| connection.provider_id == provider.id)
        .count();
    Ok((provider, count))
}

fn provider_id(configuration: &ConfigStore, id: &str) -> Result<ProviderId, String> {
    provider_and_dependents(configuration, id).map(|(provider, _)| provider.id)
}
