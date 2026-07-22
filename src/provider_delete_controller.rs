use std::rc::Rc;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

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
    let request_diagnostics = Rc::clone(&diagnostics);
    window.on_request_provider_delete(move |id| {
        request_delete(&weak, &request_config, &request_diagnostics, id);
    });

    let weak = window.as_weak();
    let confirm_config = Rc::clone(configuration);
    let confirm_diagnostics = Rc::clone(&diagnostics);
    window.on_confirm_provider_delete(move || {
        delete_provider(&weak, &confirm_config, &confirm_diagnostics);
    });

    let weak = window.as_weak();
    let cancel_config = Rc::clone(configuration);
    let cancel_diagnostics = Rc::clone(&diagnostics);
    window.on_cancel_provider_delete(move || {
        crate::provider_list_controller::show(
            &weak,
            Rc::clone(&cancel_config),
            Rc::clone(&cancel_diagnostics),
        );
    });
}

fn request_delete(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
    id: SharedString,
) {
    let Some(window) = weak.upgrade() else { return };
    match provider_and_dependents(configuration, id.as_str()) {
        Ok((provider, connections)) => {
            window.set_pending_provider_id(provider.id.as_str().into());
            window.set_pending_provider_name(provider.name.into());
            window.set_pending_connection_count(connections.len() as i32);
            window.set_pending_provider_connections(ModelRc::new(Rc::new(VecModel::from_iter(
                connections.into_iter().map(SharedString::from),
            ))));
            window.set_status_message(SharedString::default());
            window.set_page(6);
        }
        Err(_) => diagnostics_controller::present(
            &window,
            diagnostics,
            "Provider could not be prepared for deletion",
            "provider deletion lookup failed",
            "SyncPak could not prepare this provider for deletion. It may have been removed.",
        ),
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
        Ok(_) => {
            crate::provider_list_controller::show(
                weak,
                Rc::clone(configuration),
                Rc::clone(diagnostics),
            );
            window.set_notice_message("Provider deleted.".into());
        }
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
) -> Result<(crate::configuration::ProviderConfig, Vec<String>), String> {
    let config = configuration.load().map_err(|error| error.to_string())?;
    let provider = config
        .providers
        .iter()
        .find(|provider| provider.id.as_str() == id)
        .cloned()
        .ok_or_else(|| "The provider no longer exists.".to_owned())?;
    let connections = config
        .connections
        .iter()
        .filter(|connection| connection.provider_id == provider.id)
        .map(|connection| connection.name.clone())
        .collect();
    Ok((provider, connections))
}

fn provider_id(configuration: &ConfigStore, id: &str) -> Result<ProviderId, String> {
    provider_and_dependents(configuration, id).map(|(provider, _)| provider.id)
}
