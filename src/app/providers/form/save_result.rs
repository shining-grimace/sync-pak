use super::state::clear_transient_state;
use crate::{
    AppWindow,
    app::{
        diagnostics::{self as diagnostics_controller, SharedDiagnosticLog},
        onboarding::complete_welcome,
        providers::{
            bucket_cache::{self as provider_bucket_cache, ProviderBucketCache},
            list_controller as provider_list_controller,
            save_error::ProviderPersistenceError,
        },
    },
    configuration::{ConfigStore, ProviderConfig},
};
use slint::{ComponentHandle, Model};
use std::rc::Rc;

pub(super) fn complete(
    window: &AppWindow,
    configuration: Rc<ConfigStore>,
    diagnostics: SharedDiagnosticLog,
    buckets: ProviderBucketCache,
    edit_id: slint::SharedString,
    result: Result<ProviderConfig, ProviderPersistenceError>,
) {
    let weak = window.as_weak();
    match result {
        Ok(saved_provider) => {
            let save_after_verification = window.get_provider_save_after_verification();
            let buckets_after_verification =
                save_after_verification.then(|| verified_buckets(window));
            if !edit_id.is_empty() {
                provider_bucket_cache::remove(&buckets, edit_id.as_str());
            }
            if let Some(verified_buckets) = buckets_after_verification {
                provider_bucket_cache::record(
                    &buckets,
                    saved_provider.id.as_str(),
                    verified_buckets,
                );
            }
            clear_transient_state(window);
            match complete_welcome(&configuration) {
                Ok(()) => {
                    provider_list_controller::show(
                        &weak,
                        configuration,
                        Rc::clone(&diagnostics),
                        Rc::clone(&buckets),
                    );
                    window.set_notice_message("Provider saved securely.".into());
                }
                Err(_) => diagnostics_controller::present(
                    window,
                    &diagnostics,
                    "Provider was saved but welcome state could not be updated",
                    "welcome state save failed",
                    "The provider was saved, but SyncPak could not update its welcome state.",
                ),
            }
        }
        Err(error) => {
            window.set_provider_save_after_verification(false);
            window.set_page(2);
            super::credential_task::present_error(window, &diagnostics, &error);
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
