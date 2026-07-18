use std::{rc::Rc, time::Duration};

use slint::SharedString;

use crate::{
    AppWindow,
    configuration::ConfigStore,
    connection_form_data::{reset, set_provider_bucket, set_provider_models},
    diagnostics_controller::{self, SharedDiagnosticLog},
};

pub(crate) fn show_add(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    window.set_status_message(SharedString::default());
    reset(&window);
    window.set_page(5);
    let weak = weak.clone();
    slint::Timer::single_shot(Duration::ZERO, move || {
        load_providers(&weak, &configuration, &diagnostics)
    });
}

pub(crate) fn select_provider(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
    index: i32,
) {
    let Some(window) = weak.upgrade() else { return };
    match configuration.load() {
        Ok(config) => set_provider_bucket(&window, &config.providers, index),
        Err(_) => provider_load_error(&window, diagnostics),
    }
}

fn load_providers(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    match configuration.load() {
        Ok(config) => {
            set_provider_models(&window, &config.providers);
            set_provider_bucket(
                &window,
                &config.providers,
                window.get_connection_form_provider(),
            );
        }
        Err(_) => provider_load_error(&window, diagnostics),
    }
}

fn provider_load_error(window: &AppWindow, diagnostics: &SharedDiagnosticLog) {
    diagnostics_controller::present(
        window,
        diagnostics,
        "Providers could not be loaded for a connection",
        "provider configuration load failed",
        "SyncPak could not load providers. Check configuration storage and try again.",
    );
}
