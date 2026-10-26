use super::state::{begin_verification, provider_id, provider_kind};
#[cfg(not(feature = "provider-s3"))]
use crate::app::diagnostics as diagnostics_controller;
use crate::{
    AppWindow,
    app::{diagnostics::SharedDiagnosticLog, form_validation},
    configuration::{
        ConfigStore, CredentialReference, ProviderConfig, ProviderCredentials, ProviderId,
    },
};
use slint::{ModelRc, SharedString, VecModel};
use std::rc::Rc;

pub(super) fn verify(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
) {
    let Some(window) = weak.upgrade() else { return };
    if window.get_provider_storage_busy() || window.get_provider_verifying() {
        return;
    }
    super::secret_reveal::hide(&window);
    let weak = weak.clone();
    let next_configuration = Rc::clone(&configuration);
    let next_diagnostics = Rc::clone(&diagnostics);
    super::credential_task::resolve(
        &window,
        &configuration,
        diagnostics,
        move |_, credentials| {
            verify_ready(&weak, next_configuration, next_diagnostics, credentials);
        },
    );
}

fn verify_ready(
    weak: &slint::Weak<AppWindow>,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    credentials: ProviderCredentials,
) {
    let Some(window) = weak.upgrade() else { return };
    crate::app::providers::form::secret_reveal::hide(&window);
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
    if let Err(error) = form_validation::provider(form_validation::ProviderFields {
        name: &window.get_provider_form_name(),
        access_key_id: &credentials.access_key_id,
        secret_access_key: &credentials.secret_access_key,
        kind,
        account_id: &account,
        region: &region,
        default_bucket: &bucket,
        endpoint: &endpoint,
    }) {
        window.set_provider_save_after_verification(false);
        window.set_provider_validation_focus(form_validation::provider_focus(&error));
        window.set_status_message(error.into());
        return;
    }
    let options = match super::transport::options(&window, kind) {
        Ok(options) => options,
        Err(error) => {
            window.set_provider_save_after_verification(false);
            window.set_status_message(error.into());
            return;
        }
    };
    let provider = ProviderConfig {
        id: id.clone(),
        credential_reference: CredentialReference { provider_id: id },
        name: window.get_provider_form_name().to_string(),
        kind,
        options,
        verified: false,
    };
    if let Err(error) = provider
        .options
        .transport_for_bucket(provider.options.default_bucket.as_deref(), None)
        .resolve(kind, provider.options.account_id.as_deref())
    {
        window.set_provider_save_after_verification(false);
        window.set_status_message(error.into());
        return;
    }
    window.set_provider_verified_buckets(ModelRc::new(Rc::new(VecModel::default())));
    window.set_provider_bucket_list_empty(false);
    let _generation = begin_verification(&window);
    #[cfg(feature = "provider-s3")]
    crate::app::providers::s3_verification_controller::start(
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
