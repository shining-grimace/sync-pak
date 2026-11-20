use std::{rc::Rc, time::Duration};

use slint::{ModelRc, SharedString, VecModel};

use crate::{
    AppWindow,
    app::connections::form::data::{
        mark_clean, populate, reset, reset_bucket_discovery, set_provider_bucket,
        set_provider_models,
    },
    app::diagnostics::{self as diagnostics_controller, SharedDiagnosticLog},
    app::providers::bucket_cache::ProviderBucketCache,
    configuration::ConfigStore,
};

pub(crate) fn show_add(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    buckets: ProviderBucketCache,
) {
    let Some(window) = weak.upgrade() else { return };
    window.set_status_message(SharedString::default());
    reset(&window);
    window.set_connection_providers_loading(true);
    window.set_connection_providers_load_failed(false);
    window.set_page(5);
    let weak = weak.clone();
    slint::Timer::single_shot(Duration::ZERO, move || {
        load_providers(&weak, &configuration, &diagnostics, &buckets)
    });
}

pub(crate) fn select_provider(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
    buckets: &ProviderBucketCache,
    index: i32,
) {
    let Some(window) = weak.upgrade() else { return };
    match configuration.load() {
        Ok(config) => {
            super::transport::reset(&window);
            window.set_connection_form_bucket(Default::default());
            apply_provider_selection(&window, configuration, &config.providers, buckets, index);
        }
        Err(_) => provider_load_error(&window, diagnostics),
    }
}

pub(crate) fn retry_load_providers(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    buckets: ProviderBucketCache,
) {
    let Some(window) = weak.upgrade() else { return };
    if window.get_connection_providers_loading() {
        return;
    }
    window.set_connection_providers_loading(true);
    let weak = weak.clone();
    slint::Timer::single_shot(Duration::ZERO, move || {
        load_providers(&weak, &configuration, &diagnostics, &buckets)
    });
}

pub(crate) fn show_edit(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
    buckets: &ProviderBucketCache,
    id: SharedString,
) {
    let Some(window) = weak.upgrade() else { return };
    let result = configuration
        .load()
        .map_err(|error| error.to_string())
        .and_then(|config| {
            let connection = config
                .connections
                .iter()
                .find(|connection| id == connection.id.as_str())
                .cloned()
                .ok_or_else(|| "The connection no longer exists.".to_owned())?;
            let provider_index = config
                .providers
                .iter()
                .position(|provider| provider.id == connection.provider_id)
                .ok_or_else(|| "The connection's provider no longer exists.".to_owned())?;
            Ok((config.providers, connection, provider_index))
        });
    match result {
        Ok((providers, connection, provider_index)) => {
            populate(&window, &providers, connection, provider_index);
            super::bucket_discovery::load(
                &window,
                configuration,
                providers.get(provider_index),
                buckets,
            );
        }
        Err(_) => diagnostics_controller::present(
            &window,
            diagnostics,
            "Connection could not be opened",
            "connection edit load failed",
            "SyncPak could not open this connection. It may have been removed.",
        ),
    }
}

fn load_providers(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
    buckets: &ProviderBucketCache,
) {
    let Some(window) = weak.upgrade() else { return };
    match configuration.load() {
        Ok(config) => {
            window.set_connection_providers_load_failed(false);
            set_provider_models(&window, &config.providers);
            apply_provider_selection(
                &window,
                configuration,
                &config.providers,
                buckets,
                window.get_connection_form_provider(),
            );
            mark_clean(&window);
        }
        Err(_) => provider_load_error(&window, diagnostics),
    }
    window.set_connection_providers_loading(false);
}

fn apply_provider_selection(
    window: &AppWindow,
    configuration: &ConfigStore,
    providers: &[crate::configuration::ProviderConfig],
    buckets: &ProviderBucketCache,
    index: i32,
) {
    set_provider_bucket(window, providers, index);
    let provider = usize::try_from(index)
        .ok()
        .and_then(|index| providers.get(index));
    super::bucket_discovery::load(window, configuration, provider, buckets);
}

fn provider_load_error(window: &AppWindow, diagnostics: &SharedDiagnosticLog) {
    window.set_connection_providers_loading(false);
    window.set_connection_providers_load_failed(true);
    window.set_connection_form_provider(-1);
    window.set_provider_names(ModelRc::new(Rc::new(VecModel::default())));
    window.set_provider_buckets(ModelRc::new(Rc::new(VecModel::default())));
    reset_bucket_discovery(window);
    diagnostics_controller::present(
        window,
        diagnostics,
        "Providers could not be loaded for a connection",
        "provider configuration load failed",
        "SyncPak could not load providers. Check configuration storage and try again.",
    );
}
