use std::{rc::Rc, time::Duration};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{
    AppWindow, ProviderRow,
    configuration::{ConfigStore, ProviderKind},
    diagnostics_controller::{self, SharedDiagnosticLog},
};

pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let weak = window.as_weak();
    let configuration = Rc::clone(configuration);
    window
        .on_show_providers(move || show(&weak, Rc::clone(&configuration), Rc::clone(&diagnostics)));
}

pub(crate) fn show(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    window.set_status_message(SharedString::default());
    window.set_page(1);
    let weak = weak.clone();
    slint::Timer::single_shot(Duration::ZERO, move || {
        refresh(&weak, &configuration, &diagnostics)
    });
}

fn refresh(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    match configuration.load() {
        Ok(config) => {
            let rows = config.providers.into_iter().map(|provider| {
                let connection_count = config
                    .connections
                    .iter()
                    .filter(|connection| connection.provider_id == provider.id)
                    .count();
                ProviderRow {
                    id: provider.id.as_str().into(),
                    name: provider.name.into(),
                    kind: kind_name(provider.kind).into(),
                    connection_summary: connection_summary(connection_count).into(),
                }
            });
            window.set_providers(ModelRc::new(Rc::new(VecModel::from_iter(rows))));
            window.set_status_message(SharedString::default());
        }
        Err(_) => diagnostics_controller::present(
            &window,
            diagnostics,
            "Providers could not be loaded",
            "provider configuration load failed",
            "SyncPak could not load providers. Check configuration storage and try again.",
        ),
    }
}

fn connection_summary(count: usize) -> String {
    match count {
        1 => "1 connection".into(),
        _ => format!("{count} connections"),
    }
}

fn kind_name(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::CloudflareR2 => "Cloudflare R2",
        ProviderKind::BackblazeB2 => "Backblaze B2",
        ProviderKind::AwsS3 => "AWS S3",
    }
}
