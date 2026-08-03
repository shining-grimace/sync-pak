use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::{
    AppWindow,
    configuration::{
        ConfigStore, CredentialReference, ProviderConfig, ProviderDraft, ProviderId,
        ProviderRepository,
    },
    diagnostics_controller::{self, SharedDiagnosticLog},
    form_validation,
    onboarding::complete_welcome,
    platform::PlatformCredentialStore,
    provider_bucket_cache::{self, ProviderBucketCache},
    provider_form::{
        begin_verification, clear_transient_state, invalidate_verification, is_dirty, mark_clean,
        provider_id, provider_kind, provider_kind_index, provider_options,
    },
    provider_form_credentials, provider_list_controller,
    provider_save_error::ProviderPersistenceError,
};

pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    buckets: ProviderBucketCache,
) {
    let weak = window.as_weak();
    window.on_show_add_provider(move || show_add(&weak));

    let weak = window.as_weak();
    let save_configuration = Rc::clone(configuration);
    let save_diagnostics = Rc::clone(&diagnostics);
    let save_buckets = Rc::clone(&buckets);
    window.on_save_provider(move |name, kind, access_key_id, secret_access_key| {
        save(
            &weak,
            Rc::clone(&save_configuration),
            &save_diagnostics,
            &save_buckets,
            name,
            kind,
            access_key_id,
            secret_access_key,
        )
    });

    let weak = window.as_weak();
    let verify_configuration = Rc::clone(configuration);
    let verify_diagnostics = Rc::clone(&diagnostics);
    window.on_verify_provider(move || {
        if let Some(window) = weak.upgrade() {
            window.set_provider_save_after_verification(false);
        }
        verify(
            &weak,
            Rc::clone(&verify_configuration),
            Rc::clone(&verify_diagnostics),
        );
    });

    let weak = window.as_weak();
    let save_and_verify_configuration = Rc::clone(configuration);
    let save_and_verify_diagnostics = Rc::clone(&diagnostics);
    window.on_save_and_verify_provider(move || {
        if let Some(window) = weak.upgrade() {
            window.set_provider_save_after_verification(true);
        }
        verify(
            &weak,
            Rc::clone(&save_and_verify_configuration),
            Rc::clone(&save_and_verify_diagnostics),
        );
    });

    let weak = window.as_weak();
    window.on_request_save_provider(move || {
        if let Some(window) = weak.upgrade() {
            crate::provider_secret_reveal_controller::hide(&window);
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
    window.on_request_discard_provider(move || request_discard(&weak));

    let weak = window.as_weak();
    window.on_cancel_discard_provider(move || {
        if let Some(window) = weak.upgrade() {
            window.set_pending_navigation_page(-1);
            window.set_page(2);
        }
    });

    let weak = window.as_weak();
    window.on_confirm_discard_provider(move || {
        if let Some(window) = weak.upgrade() {
            clear_transient_state(&window);
            window.invoke_complete_pending_navigation();
        }
    });

    let weak = window.as_weak();
    let edit_configuration = Rc::clone(configuration);
    window.on_request_provider_edit(move |id| edit(&weak, &edit_configuration, &diagnostics, id));
}

fn verify(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    crate::provider_secret_reveal_controller::hide(&window);
    window.set_status_message(SharedString::default());
    window.set_notice_message(SharedString::default());
    let Some(kind) = provider_kind(window.get_provider_form_kind()) else {
        window.set_provider_save_after_verification(false);
        return;
    };
    let account = window.get_provider_form_account_id();
    let region = window.get_provider_form_region();
    let bucket = window.get_provider_form_bucket();
    let endpoint = window.get_provider_form_endpoint();
    let access = window.get_provider_form_access_key();
    let secret = window.get_provider_form_secret_key();
    let session_token = window.get_provider_form_session_token();
    let edit_id = window.get_provider_form_id();
    let id = if edit_id.is_empty() {
        ProviderId::new()
    } else {
        match provider_id(&configuration, edit_id.as_str()) {
            Ok(id) => id,
            Err(_) => {
                window.set_provider_save_after_verification(false);
                return;
            }
        }
    };
    let credentials = match provider_form_credentials::resolve(
        &configuration,
        (!edit_id.is_empty()).then_some(&id),
        &access,
        &secret,
        &session_token,
    ) {
        Ok(credentials) => credentials,
        Err(error) => {
            window.set_provider_save_after_verification(false);
            present_credential_load_error(&window, &diagnostics, &error);
            return;
        }
    };
    if let Err(error) = form_validation::provider(
        &window.get_provider_form_name(),
        &credentials.access_key_id,
        &credentials.secret_access_key,
        kind,
        &account,
        &region,
        &bucket,
        &endpoint,
    ) {
        window.set_provider_save_after_verification(false);
        window.set_status_message(error.into());
        return;
    }
    let provider = ProviderConfig {
        id: id.clone(),
        credential_reference: CredentialReference { provider_id: id },
        name: window.get_provider_form_name().to_string(),
        kind,
        options: provider_options(&account, &region, &bucket, &endpoint),
        verified: false,
    };
    window.set_provider_verified_buckets(ModelRc::new(Rc::new(VecModel::default())));
    window.set_provider_bucket_list_empty(false);
    let _generation = begin_verification(&window);
    #[cfg(feature = "provider-s3")]
    crate::s3_provider_verify_controller::start(
        weak.clone(),
        provider,
        credentials,
        diagnostics,
        _generation,
    );
    #[cfg(not(feature = "provider-s3"))]
    {
        let _ = (provider, credentials);
        window.set_provider_verifying(false);
        window.set_provider_save_after_verification(false);
        window.set_provider_bucket_list_empty(false);
        diagnostics_controller::present(
            &window,
            &diagnostics,
            "Provider could not be verified",
            "provider support unavailable",
            "This build cannot verify cloud providers.",
        );
    }
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
        window.set_provider_form_endpoint(SharedString::default());
        clear_transient_state(&window);
        mark_clean(&window);
        window.set_page(2);
    }
}

fn request_discard(weak: &slint::Weak<AppWindow>) {
    let Some(window) = weak.upgrade() else { return };
    crate::provider_secret_reveal_controller::hide(&window);
    invalidate_verification(&window);
    if window.get_pending_navigation_page() < 0 {
        window.set_pending_navigation_page(1);
    }
    if is_dirty(&window) {
        window.set_page(15);
    } else {
        clear_transient_state(&window);
        window.invoke_complete_pending_navigation();
    }
}

fn save(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: &SharedDiagnosticLog,
    buckets: &ProviderBucketCache,
    name: SharedString,
    kind: i32,
    access_key_id: SharedString,
    secret_access_key: SharedString,
) {
    let Some(window) = weak.upgrade() else { return };
    crate::provider_secret_reveal_controller::hide(&window);
    let Some(kind) = provider_kind(kind) else {
        window.set_status_message("Choose a provider type.".into());
        return;
    };
    let account_id = window.get_provider_form_account_id();
    let region = window.get_provider_form_region();
    let default_bucket = window.get_provider_form_bucket();
    let endpoint = window.get_provider_form_endpoint();
    let session_token = window.get_provider_form_session_token();
    let edit_id = window.get_provider_form_id();
    let edit_provider_id = if edit_id.is_empty() {
        None
    } else {
        match provider_id(&configuration, edit_id.as_str()) {
            Ok(id) => Some(id),
            Err(_) => {
                window.set_page(2);
                window.set_status_message("The provider no longer exists.".into());
                return;
            }
        }
    };
    let credentials = match provider_form_credentials::resolve(
        &configuration,
        edit_provider_id.as_ref(),
        &access_key_id,
        &secret_access_key,
        &session_token,
    ) {
        Ok(credentials) => credentials,
        Err(error) => {
            window.set_page(2);
            present_credential_load_error(&window, diagnostics, &error);
            return;
        }
    };
    if let Err(error) = form_validation::provider(
        &name,
        &credentials.access_key_id,
        &credentials.secret_access_key,
        kind,
        &account_id,
        &region,
        &default_bucket,
        &endpoint,
    ) {
        window.set_page(2);
        window.set_status_message(error.into());
        return;
    }
    let verified = window.get_provider_save_after_verification()
        || edit_provider_id
            .as_ref()
            .is_some_and(|id| !is_dirty(&window) && was_verified(&configuration, id));
    let draft = ProviderDraft {
        name: name.to_string(),
        kind,
        options: provider_options(&account_id, &region, &default_bucket, &endpoint),
        verified,
    };
    let result = (|| -> Result<_, ProviderPersistenceError> {
        let store =
            PlatformCredentialStore::new().map_err(ProviderPersistenceError::ProtectedStore)?;
        let repository = ProviderRepository::new(&configuration, &store);
        if edit_id.is_empty() {
            repository
                .create(draft, &credentials)
                .map_err(ProviderPersistenceError::from)
        } else {
            repository
                .update(
                    edit_provider_id
                        .as_ref()
                        .ok_or(ProviderPersistenceError::Other)?,
                    draft,
                    &credentials,
                )
                .map_err(ProviderPersistenceError::from)
        }
    })();
    match result {
        Ok(saved_provider) => {
            let save_after_verification = window.get_provider_save_after_verification();
            let buckets_after_verification =
                save_after_verification.then(|| verified_buckets(&window));
            if !edit_id.is_empty() {
                provider_bucket_cache::remove(buckets, edit_id.as_str());
            }
            if let Some(verified_buckets) = buckets_after_verification {
                provider_bucket_cache::record(
                    buckets,
                    saved_provider.id.as_str(),
                    verified_buckets,
                );
            }
            clear_transient_state(&window);
            match complete_welcome(&configuration) {
                Ok(()) => {
                    provider_list_controller::show(
                        weak,
                        configuration,
                        Rc::clone(diagnostics),
                        Rc::clone(buckets),
                    );
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
        Err(error) => {
            window.set_provider_save_after_verification(false);
            window.set_page(2);
            let (summary, technical_details, message) = error.save_presentation();
            diagnostics_controller::present(
                &window,
                diagnostics,
                summary,
                technical_details,
                message,
            );
        }
    }
}

fn verified_buckets(window: &AppWindow) -> Vec<String> {
    window
        .get_provider_verified_buckets()
        .iter()
        .map(|bucket| bucket.to_string())
        .collect()
}

fn was_verified(configuration: &ConfigStore, provider_id: &ProviderId) -> bool {
    configuration.load().is_ok_and(|config| {
        config
            .providers
            .iter()
            .any(|provider| provider.id == *provider_id && provider.verified)
    })
}

fn present_credential_load_error(
    window: &AppWindow,
    diagnostics: &SharedDiagnosticLog,
    error: &ProviderPersistenceError,
) {
    let (_, technical_details, _) = error.save_presentation();
    diagnostics_controller::present(
        window,
        diagnostics,
        "Saved credentials could not be accessed",
        technical_details,
        "Unlock this device's protected credential storage, then try again.",
    );
}

fn edit(
    weak: &slint::Weak<AppWindow>,
    configuration: &ConfigStore,
    diagnostics: &SharedDiagnosticLog,
    id: SharedString,
) {
    let Some(window) = weak.upgrade() else { return };
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
            let credentials = match provider_form_credentials::load(configuration, &provider.id) {
                Ok(credentials) => credentials,
                Err(error) => {
                    present_credential_load_error(&window, diagnostics, &error);
                    return;
                }
            };
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
            window.set_provider_form_access_key(credentials.access_key_id.into());
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
