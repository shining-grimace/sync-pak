use std::{rc::Rc, sync::Arc};

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

use crate::{
    AppWindow,
    configuration::{ConfigStore, ProviderId, ProviderRepository},
    diagnostics_controller::{self, SharedDiagnosticLog},
    operation_cancellation::ConnectionOperationCanceller,
    platform::PlatformCredentialStore,
    provider_bucket_cache::{self, ProviderBucketCache},
    provider_save_error::ProviderPersistenceError,
};

pub(crate) fn configure(
    window: &AppWindow,
    configuration: &Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    buckets: ProviderBucketCache,
    canceller: Option<Arc<dyn ConnectionOperationCanceller + Send + Sync>>,
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
    let confirm_buckets = Rc::clone(&buckets);
    let confirm_canceller = canceller.clone();
    window.on_confirm_provider_delete(move || {
        delete_provider(
            &weak,
            &confirm_config,
            &confirm_diagnostics,
            &confirm_buckets,
            confirm_canceller.as_deref(),
        );
    });

    let weak = window.as_weak();
    let cancel_config = Rc::clone(configuration);
    let cancel_diagnostics = Rc::clone(&diagnostics);
    let cancel_buckets = Rc::clone(&buckets);
    window.on_cancel_provider_delete(move || {
        crate::provider_list_controller::show(
            &weak,
            Rc::clone(&cancel_config),
            Rc::clone(&cancel_diagnostics),
            Rc::clone(&cancel_buckets),
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
    buckets: &ProviderBucketCache,
    canceller: Option<&(dyn ConnectionOperationCanceller + Send + Sync)>,
) {
    let Some(window) = weak.upgrade() else { return };
    let pending_id = window.get_pending_provider_id();
    let result = (|| -> Result<_, ProviderPersistenceError> {
        let id = provider_id(configuration, pending_id.as_str())
            .map_err(|_| ProviderPersistenceError::Other)?;
        let store =
            PlatformCredentialStore::new().map_err(ProviderPersistenceError::ProtectedStore)?;
        match canceller {
            Some(canceller) => crate::operation_cancellation::delete_provider(
                configuration,
                &store,
                canceller,
                &id,
            )
            .map(|_| ())
            .map_err(ProviderPersistenceError::from),
            None => ProviderRepository::new(configuration, &store)
                .delete(&id)
                .map(|_| ())
                .map_err(ProviderPersistenceError::from),
        }
    })();
    match result {
        Ok(_) => {
            provider_bucket_cache::remove(buckets, pending_id.as_str());
            crate::provider_list_controller::show(
                weak,
                Rc::clone(configuration),
                Rc::clone(diagnostics),
                Rc::clone(buckets),
            );
            window.set_notice_message("Provider deleted.".into());
        }
        Err(error) => {
            let (summary, technical_details, message) = error.delete_presentation();
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
