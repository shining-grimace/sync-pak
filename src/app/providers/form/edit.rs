use super::{
    credentials as provider_form_credentials,
    state::{clear_transient_state, mark_clean, provider_kind_index},
};
use crate::{
    AppWindow,
    app::diagnostics::{self as diagnostics_controller, SharedDiagnosticLog},
    configuration::ConfigStore,
};
use slint::SharedString;
use std::rc::Rc;

pub(super) fn edit(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
    id: SharedString,
) {
    let Some(window) = weak.upgrade() else { return };
    if window.get_provider_storage_busy() {
        return;
    }
    clear_transient_state(&window);
    match configuration
        .load()
        .map_err(|error| error.to_string())
        .and_then(|config| {
            config
                .providers
                .into_iter()
                .find(|provider| id == provider.id.as_str())
                .ok_or_else(|| "The provider no longer exists.".to_owned())
        }) {
        Ok(provider) => {
            let worker_configuration = ConfigStore::at(configuration.path().to_owned());
            let worker_id = provider.id.clone();
            let diagnostics = Rc::clone(diagnostics);
            super::storage_task::start(
                &window,
                move |interaction| {
                    provider_form_credentials::load(&worker_configuration, &worker_id, interaction)
                },
                move |window, result| {
                    let credentials = match result {
                        Ok(credentials) => credentials,
                        Err(error) => {
                            super::credential_task::present_error(window, &diagnostics, &error);
                            return;
                        }
                    };
                    super::transport::populate(window, &provider.options);
                    window.set_provider_form_id(provider.id.as_str().into());
                    window.set_provider_form_name(provider.name.into());
                    window.set_provider_form_kind(provider_kind_index(provider.kind));
                    window.set_provider_form_account_id(
                        provider.options.account_id.unwrap_or_default().into(),
                    );
                    window.set_provider_form_region(
                        provider.options.region.unwrap_or_default().into(),
                    );
                    window.set_provider_form_bucket(
                        provider.options.default_bucket.unwrap_or_default().into(),
                    );
                    window.set_provider_form_endpoint(
                        provider.options.endpoint.unwrap_or_default().into(),
                    );
                    window.set_provider_form_access_key(credentials.access_key_id.into());
                    mark_clean(window);
                    window.set_status_message(SharedString::default());
                    window.set_page(2);
                },
            );
        }
        Err(_) => diagnostics_controller::present(
            &window,
            diagnostics,
            "Provider could not be opened",
            "provider edit load failed",
            "SyncPak could not open this provider. It may have been removed.",
        ),
    }
}
