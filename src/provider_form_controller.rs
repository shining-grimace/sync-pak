use std::rc::Rc;

use slint::{ComponentHandle, SharedString};

use crate::{
    AppWindow,
    configuration::{ConfigStore, ProviderCredentials, ProviderDraft, ProviderRepository},
    diagnostics_controller::{self, SharedDiagnosticLog},
    form_validation,
    onboarding::complete_welcome,
    platform::PlatformCredentialStore,
    provider_form::{
        is_dirty, mark_clean, provider_id, provider_kind, provider_kind_index, provider_options,
    },
    provider_list_controller,
};

pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let weak = window.as_weak();
    window.on_show_add_provider(move || show_add(&weak));

    let weak = window.as_weak();
    let save_configuration = Rc::clone(configuration);
    let save_diagnostics = Rc::clone(&diagnostics);
    window.on_save_provider(move |name, kind, access_key_id, secret_access_key| {
        save(
            &weak,
            Rc::clone(&save_configuration),
            &save_diagnostics,
            name,
            kind,
            access_key_id,
            secret_access_key,
        )
    });

    let weak = window.as_weak();
    window.on_request_save_provider(move || {
        if let Some(window) = weak.upgrade() {
            window.set_status_message(SharedString::default());
            window.set_page(13);
        }
    });

    let weak = window.as_weak();
    window.on_cancel_save_provider(move || {
        if let Some(window) = weak.upgrade() {
            window.set_page(2);
        }
    });

    let weak = window.as_weak();
    let discard_configuration = Rc::clone(configuration);
    let discard_diagnostics = Rc::clone(&diagnostics);
    window.on_request_discard_provider(move || {
        request_discard(
            &weak,
            Rc::clone(&discard_configuration),
            Rc::clone(&discard_diagnostics),
        );
    });

    let weak = window.as_weak();
    window.on_cancel_discard_provider(move || {
        if let Some(window) = weak.upgrade() {
            window.set_page(2);
        }
    });

    let weak = window.as_weak();
    let discard_configuration = Rc::clone(configuration);
    let discard_diagnostics = Rc::clone(&diagnostics);
    window.on_confirm_discard_provider(move || {
        provider_list_controller::show(
            &weak,
            Rc::clone(&discard_configuration),
            Rc::clone(&discard_diagnostics),
        );
    });

    let weak = window.as_weak();
    let edit_configuration = Rc::clone(configuration);
    window.on_request_provider_edit(move |id| edit(&weak, &edit_configuration, &diagnostics, id));
}

fn show_add(weak: &slint::Weak<AppWindow>) {
    if let Some(window) = weak.upgrade() {
        window.set_status_message(SharedString::default());
        window.set_provider_form_id(SharedString::default());
        window.set_provider_form_name(SharedString::default());
        window.set_provider_form_kind(0);
        window.set_provider_form_account_id(SharedString::default());
        window.set_provider_form_region(SharedString::default());
        window.set_provider_form_bucket(SharedString::default());
        window.set_provider_form_access_key(SharedString::default());
        window.set_provider_form_secret_key(SharedString::default());
        window.set_provider_form_endpoint(SharedString::default());
        window.set_provider_form_session_token(SharedString::default());
        window.set_provider_secret_visible(false);
        window.set_provider_advanced_expanded(false);
        mark_clean(&window);
        window.set_page(2);
    }
}

fn request_discard(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    if is_dirty(&window) {
        window.set_page(15);
    } else {
        provider_list_controller::show(weak, configuration, diagnostics);
    }
}

fn save(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: &SharedDiagnosticLog,
    name: SharedString,
    kind: i32,
    access_key_id: SharedString,
    secret_access_key: SharedString,
) {
    let Some(window) = weak.upgrade() else { return };
    let Some(kind) = provider_kind(kind) else {
        window.set_status_message("Choose a provider type.".into());
        return;
    };
    let account_id = window.get_provider_form_account_id();
    let region = window.get_provider_form_region();
    let default_bucket = window.get_provider_form_bucket();
    let endpoint = window.get_provider_form_endpoint();
    let session_token = window.get_provider_form_session_token();
    if let Err(error) = form_validation::provider(
        &name,
        &access_key_id,
        &secret_access_key,
        kind,
        &account_id,
        &region,
        &default_bucket,
    ) {
        window.set_page(2);
        window.set_status_message(error.into());
        return;
    }
    let credentials = ProviderCredentials {
        access_key_id: access_key_id.to_string(),
        secret_access_key: secret_access_key.to_string(),
        session_token: (!session_token.trim().is_empty()).then(|| session_token.to_string()),
    };
    let draft = ProviderDraft {
        name: name.to_string(),
        kind,
        options: provider_options(&account_id, &region, &default_bucket, &endpoint),
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
            repository
                .update(
                    &provider_id(&configuration, edit_id.as_str())?,
                    draft,
                    &credentials,
                )
                .map_err(|error| error.to_string())
        }
    })();
    match result {
        Ok(_) => {
            window.set_provider_form_access_key(SharedString::default());
            window.set_provider_form_secret_key(SharedString::default());
            window.set_provider_form_session_token(SharedString::default());
            window.set_provider_secret_visible(false);
            window.set_provider_advanced_expanded(false);
            match complete_welcome(&configuration) {
                Ok(()) => {
                    provider_list_controller::show(weak, configuration, Rc::clone(diagnostics));
                    window.set_notice_message("Provider saved securely.".into());
                }
                Err(_) => diagnostics_controller::present(
                    &window,
                    diagnostics,
                    "Provider was saved but welcome state could not be updated",
                    "welcome state save failed",
                    "The provider was saved, but SyncPak could not update its welcome state.",
                ),
            }
        }
        Err(_) => {
            window.set_page(2);
            diagnostics_controller::present(
                &window,
                diagnostics,
                "Provider settings could not be saved",
                "provider save failed",
                "SyncPak could not save this provider. Check its settings and protected storage, then try again.",
            );
        }
    }
}

fn edit(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
    id: SharedString,
) {
    let Some(window) = weak.upgrade() else { return };
    window.set_provider_secret_visible(false);
    window.set_provider_form_access_key(SharedString::default());
    window.set_provider_form_secret_key(SharedString::default());
    window.set_provider_form_session_token(SharedString::default());
    window.set_provider_advanced_expanded(false);
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
            window.set_provider_form_endpoint(provider.options.endpoint.unwrap_or_default().into());
            mark_clean(&window);
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
