use std::{rc::Rc, time::Duration};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{
    AppWindow, ProviderRow,
    configuration::{
        ConfigStore, ProviderCredentials, ProviderDraft, ProviderKind, ProviderRepository,
        StructuredError,
    },
    diagnostics_controller::{self, SharedDiagnosticLog},
    form_validation,
    onboarding::complete_welcome,
    platform::PlatformCredentialStore,
    provider_form::{provider_id, provider_kind, provider_kind_index, provider_options},
};

pub(crate) fn initialize(window: &AppWindow) {
    let diagnostics = Rc::new(std::cell::RefCell::new(Default::default()));
    diagnostics_controller::configure(window, Rc::clone(&diagnostics));
    let configuration = match ConfigStore::for_current_platform() {
        Ok(configuration) => Rc::new(configuration),
        Err(error) => {
            diagnostics_controller::record(
                &diagnostics,
                StructuredError::new(
                    "Configuration could not be opened",
                    "configuration directory unavailable",
                ),
            );
            let _ = error;
            window.set_status_message("SyncPak could not access its configuration. Check its storage location and try again.".into());
            return;
        }
    };
    configure_navigation(window, &configuration, Rc::clone(&diagnostics));
    configure_save_provider(window, &configuration, Rc::clone(&diagnostics));
    crate::provider_delete_controller::configure(window, &configuration, Rc::clone(&diagnostics));
    crate::connection_controller::configure(window, &configuration, Rc::clone(&diagnostics));
    crate::connection_delete_controller::configure(window, &configuration, Rc::clone(&diagnostics));
    crate::folder_picker_controller::configure(window, diagnostics.clone());
    match configuration.load() {
        Ok(config) if config.welcome_completed => {
            show_providers(&window.as_weak(), configuration, diagnostics)
        }
        Ok(_) => {}
        Err(error) => {
            record_configuration_load_error(&diagnostics);
            let _ = error;
            window.set_status_message(
                "SyncPak could not load its configuration. Check the file and try again.".into(),
            );
        }
    }
}

fn record_configuration_load_error(diagnostics: &SharedDiagnosticLog) {
    diagnostics_controller::record(
        diagnostics,
        StructuredError::new(
            "Configuration could not be loaded",
            "configuration load failed",
        ),
    );
}

fn configure_navigation(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let weak = window.as_weak();
    let providers_config = Rc::clone(configuration);
    let providers_diagnostics = Rc::clone(&diagnostics);
    window.on_show_providers(move || {
        show_providers(
            &weak,
            Rc::clone(&providers_config),
            Rc::clone(&providers_diagnostics),
        )
    });

    let weak = window.as_weak();
    window.on_show_add_provider(move || show_add_provider(&weak));

    let weak = window.as_weak();
    let edit_config = Rc::clone(configuration);
    let edit_diagnostics = Rc::clone(&diagnostics);
    window.on_request_provider_edit(move |id| {
        request_provider_edit(&weak, &edit_config, &edit_diagnostics, id)
    });

    let weak = window.as_weak();
    window.on_show_welcome(move || set_page(&weak, 0));

    let weak = window.as_weak();
    window.on_show_privacy(move || set_page(&weak, 3));
}

fn configure_save_provider(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let weak = window.as_weak();
    let configuration = Rc::clone(configuration);
    window.on_save_provider(move |name, kind, access_key_id, secret_access_key| {
        save_provider(
            &weak,
            Rc::clone(&configuration),
            name,
            kind,
            access_key_id,
            secret_access_key,
            &diagnostics,
        );
    });
}

fn save_provider(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    name: SharedString,
    kind: i32,
    access_key_id: SharedString,
    secret_access_key: SharedString,
    diagnostics: &SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    let Some(kind) = provider_kind(kind) else {
        window.set_status_message("Choose a provider type.".into());
        return;
    };
    let account_id = window.get_provider_form_account_id();
    let region = window.get_provider_form_region();
    let default_bucket = window.get_provider_form_bucket();
    if let Err(error) = form_validation::provider(
        &name,
        &access_key_id,
        &secret_access_key,
        kind,
        &account_id,
        &region,
        &default_bucket,
    ) {
        window.set_status_message(error.into());
        return;
    }
    let credentials = ProviderCredentials {
        access_key_id: access_key_id.to_string(),
        secret_access_key: secret_access_key.to_string(),
        session_token: None,
    };
    let draft = ProviderDraft {
        name: name.to_string(),
        kind,
        options: provider_options(&account_id, &region, &default_bucket),
    };
    let edit_id = window.get_provider_form_id();
    let result = (|| {
        let store = PlatformCredentialStore::new().map_err(|error| error.to_string())?;
        let repository = ProviderRepository::new(&configuration, &store);
        if edit_id.is_empty() {
            repository
                .create(draft, &credentials)
                .map_err(|error| error.to_string())
        } else {
            let id = provider_id(&configuration, edit_id.as_str())?;
            repository
                .update(&id, draft, &credentials)
                .map_err(|error| error.to_string())
        }
    })();
    match result {
        Ok(_) => match complete_welcome(&configuration) {
            Ok(()) => show_providers(weak, configuration, Rc::clone(diagnostics)),
            Err(_) => diagnostics_controller::present(
                &window,
                diagnostics,
                "Provider was saved but welcome state could not be updated",
                "welcome state save failed",
                "The provider was saved, but SyncPak could not update its welcome state.",
            ),
        },
        Err(_) => diagnostics_controller::present(
            &window,
            diagnostics,
            "Provider settings could not be saved",
            "provider save failed",
            "SyncPak could not save this provider. Check its settings and protected storage, then try again.",
        ),
    }
}

pub(crate) fn show_providers(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    window.set_status_message(SharedString::default());
    window.set_page(1);
    let weak = weak.clone();
    slint::Timer::single_shot(Duration::ZERO, move || {
        refresh_providers(&weak, &configuration, &diagnostics)
    });
}

fn refresh_providers(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    match configuration.load() {
        Ok(config) => {
            let rows = config.providers.into_iter().map(|provider| ProviderRow {
                id: provider.id.as_str().into(),
                name: provider.name.into(),
                kind: provider_kind_name(provider.kind).into(),
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

fn show_add_provider(weak: &slint::Weak<AppWindow>) {
    if let Some(window) = weak.upgrade() {
        window.set_status_message(SharedString::default());
        window.set_provider_form_id(SharedString::default());
        window.set_provider_form_name(SharedString::default());
        window.set_provider_form_kind(0);
        window.set_provider_form_account_id(SharedString::default());
        window.set_provider_form_region(SharedString::default());
        window.set_provider_form_bucket(SharedString::default());
        window.set_page(2);
    }
}

fn request_provider_edit(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
    id: SharedString,
) {
    let Some(window) = weak.upgrade() else { return };
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
            window.set_provider_form_id(provider.id.as_str().into());
            window.set_provider_form_name(provider.name.into());
            window.set_provider_form_kind(provider_kind_index(provider.kind));
            window.set_provider_form_account_id(
                provider.options.account_id.unwrap_or_default().into(),
            );
            window.set_provider_form_region(provider.options.region.unwrap_or_default().into());
            window.set_provider_form_bucket(
                provider.options.default_bucket.unwrap_or_default().into(),
            );
            window.set_status_message(SharedString::default());
            window.set_page(2);
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

fn set_page(weak: &slint::Weak<AppWindow>, page: i32) {
    if let Some(window) = weak.upgrade() {
        window.set_status_message(SharedString::default());
        window.set_page(page);
    }
}

fn provider_kind_name(kind: ProviderKind) -> &'static str {
    match kind {
        ProviderKind::CloudflareR2 => "Cloudflare R2",
        ProviderKind::BackblazeB2 => "Backblaze B2",
        ProviderKind::AwsS3 => "AWS S3",
    }
}
