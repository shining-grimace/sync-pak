use super::state::{is_dirty, provider_id, provider_kind};
use crate::{
    AppWindow,
    app::{
        diagnostics::SharedDiagnosticLog,
        form_validation,
        providers::{bucket_cache::ProviderBucketCache, save_error::ProviderPersistenceError},
    },
    configuration::{
        ConfigStore, ProviderCredentials, ProviderDraft, ProviderId, ProviderRepository,
    },
    platform::PlatformCredentialStore,
};
use slint::SharedString;
use std::rc::Rc;

pub(super) fn save(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: &SharedDiagnosticLog,
    buckets: &ProviderBucketCache,
    name: SharedString,
    kind: i32,
) {
    let Some(window) = weak.upgrade() else { return };
    if window.get_provider_storage_busy() || window.get_provider_verifying() {
        return;
    }
    super::secret_reveal::hide(&window);
    let weak = weak.clone();
    let next_configuration = Rc::clone(&configuration);
    let next_diagnostics = Rc::clone(diagnostics);
    let next_buckets = Rc::clone(buckets);
    super::credential_task::resolve(
        &window,
        &configuration,
        Rc::clone(diagnostics),
        move |_, credentials| {
            save_ready(
                &weak,
                next_configuration,
                &next_diagnostics,
                &next_buckets,
                name,
                kind,
                credentials,
            );
        },
    );
}

fn save_ready(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: &SharedDiagnosticLog,
    buckets: &ProviderBucketCache,
    name: SharedString,
    kind: i32,
    credentials: ProviderCredentials,
) {
    let Some(window) = weak.upgrade() else { return };
    crate::app::providers::form::secret_reveal::hide(&window);
    let Some(kind) = provider_kind(kind) else {
        window.set_status_message("Choose a provider type.".into());
        return;
    };
    let account_id = window.get_provider_form_account_id();
    let region = window.get_provider_form_region();
    let default_bucket = window.get_provider_form_bucket();
    let endpoint = window.get_provider_form_endpoint();
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
    if let Err(error) = form_validation::provider(form_validation::ProviderFields {
        name: &name,
        access_key_id: &credentials.access_key_id,
        secret_access_key: &credentials.secret_access_key,
        kind,
        account_id: &account_id,
        region: &region,
        default_bucket: &default_bucket,
        endpoint: &endpoint,
    }) {
        window.set_page(2);
        window.set_provider_validation_focus(form_validation::provider_focus(&error));
        window.set_status_message(error.into());
        return;
    }
    let verified = window.get_provider_save_after_verification()
        || edit_provider_id
            .as_ref()
            .is_some_and(|id| !is_dirty(&window) && was_verified(&configuration, id));
    let options = match super::transport::options(&window, kind) {
        Ok(options) => options,
        Err(error) => {
            window.set_page(2);
            window.set_provider_save_after_verification(false);
            window.set_status_message(error.into());
            return;
        }
    };
    let draft = ProviderDraft {
        name: name.to_string(),
        kind,
        options,
        verified,
    };
    let diagnostics = Rc::clone(diagnostics);
    let buckets = Rc::clone(buckets);
    let worker_configuration = ConfigStore::at(configuration.path().to_owned());
    let worker_edit_id = edit_provider_id.clone();
    super::storage_task::start(
        &window,
        move |interaction| {
            let store = PlatformCredentialStore::with_interaction(interaction)
                .map_err(ProviderPersistenceError::ProtectedStore)?;
            let repository = ProviderRepository::new(&worker_configuration, &store);
            if worker_edit_id.is_none() {
                repository
                    .create(draft, &credentials)
                    .map_err(ProviderPersistenceError::from)
            } else {
                repository
                    .update(
                        worker_edit_id
                            .as_ref()
                            .ok_or(ProviderPersistenceError::Other)?,
                        draft,
                        &credentials,
                    )
                    .map_err(ProviderPersistenceError::from)
            }
        },
        move |window, result| {
            super::save_result::complete(
                window,
                configuration,
                diagnostics,
                buckets,
                edit_id,
                result,
            );
        },
    );
}

fn was_verified(configuration: &ConfigStore, provider_id: &ProviderId) -> bool {
    configuration.load().is_ok_and(|config| {
        config
            .providers
            .iter()
            .any(|provider| provider.id == *provider_id && provider.verified)
    })
}
