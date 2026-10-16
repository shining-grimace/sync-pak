use std::time::Duration;

use crate::{
    capabilities::CapabilityError,
    configuration::{ConfigStore, ProviderRepository, SyncMode},
    operations::cancellation::CancellationToken,
    operations::execution::ExecutionResult,
    operations::retry::{RetryPolicy, RetrySleeper},
    operations::transfer::local_remote::LocalRemoteTransfer,
    operations::transfer::modes::add_only::execute_add_only_actions,
    operations::transfer::modes::mirror::{MirrorExecutionError, execute_confirmed_mirror},
    operations::transfer::paths::{LocalTransferRoot, RemoteTransferPrefix},
    operations::transfer::progress::TransferProgressObserver,
    platform::PlatformCredentialStore,
    providers::s3::operations::failure::{
        credential_store_message, failed_action, inventory_message, provider_message,
    },
    providers::s3::preflight::{S3PreflightError, collect_s3_connection_preflight},
    providers::s3::transport::S3Transport,
};

pub(super) async fn execute_request(
    request: &crate::operations::request::RunRequest,
    reviewed: Option<&crate::preflight::reviewed_operation::ConfirmedOperation>,
    configuration_path: &std::path::Path,
    cancellation: &CancellationToken,
    observer: &dyn TransferProgressObserver,
    jitter_seed: u64,
) -> Result<ExecutionResult, CapabilityError> {
    let configuration = ConfigStore::at(configuration_path.into());
    let credentials = match PlatformCredentialStore::new() {
        Ok(credentials) => credentials,
        Err(error) => return Ok(failed(credential_store_message(error))),
    };
    let providers = ProviderRepository::new(&configuration, &credentials);
    let current = match collect_s3_connection_preflight(
        request,
        &providers,
        local_case_sensitivity(),
        configuration_path,
    )
    .await
    {
        Ok(current) => current,
        Err(S3PreflightError::Credentials(_)) => {
            return Ok(failed(
                "SyncPak could not reload the saved credentials while rechecking this run. Unlock protected storage, then try again.",
            ));
        }
        Err(S3PreflightError::Provider(error)) => return Ok(failed(provider_message(error))),
        Err(S3PreflightError::Inventory(error)) => return Ok(failed(inventory_message(error))),
    };
    if reviewed.is_some_and(|confirmed| current != *confirmed.preflight().preflight()) {
        return Ok(failed(
            "Files changed since this review. Refresh the review before starting the operation.",
        ));
    }
    if cancellation.is_cancelled() {
        return Ok(ExecutionResult::cancelled_before_start());
    }
    let automatic;
    let confirmed = match reviewed {
        Some(confirmed) => confirmed,
        None => {
            automatic = crate::preflight::reviewed_operation::ConfirmedOperation::from_automatic(
                request.clone(),
                current,
            )
            .map_err(|_| CapabilityError::InvalidReference)?;
            &automatic
        }
    };
    let provider_credentials = match providers.load_credentials(&confirmed.request().provider.id) {
        Ok(credentials) => credentials,
        Err(_) => {
            return Ok(failed(
                "SyncPak could not access the saved credentials. Unlock protected storage, then try again.",
            ));
        }
    };
    let transport = match S3Transport::connect_bucket(
        &confirmed.request().provider,
        provider_credentials,
        &confirmed.request().connection.bucket,
        confirmed.request().connection.transport.as_ref(),
    )
    .await
    {
        Ok(transport) => transport,
        Err(error) => return Ok(failed(provider_message(error))),
    };
    let retry = RetryPolicy::default();
    let sleeper = TokioSleeper;
    let cache = crate::sync_cache::SyncCache::for_configuration(configuration_path);
    let namespace = cache
        .as_ref()
        .map(|cache| cache.namespace(confirmed.request()));
    let transfer = LocalRemoteTransfer::new(
        &transport,
        &confirmed.request().connection.bucket,
        LocalTransferRoot::from_config(&confirmed.request().connection.local_path),
        RemoteTransferPrefix::new(&confirmed.request().connection.remote_path)
            .map_err(|_| CapabilityError::InvalidReference)?,
        &retry,
        &sleeper,
    );
    let transfer = match cache.zip(namespace) {
        Some((cache, namespace)) => transfer.with_cache(cache, namespace),
        None => transfer,
    };
    let history_directory = configuration_path
        .parent()
        .ok_or(CapabilityError::Unavailable)?;
    let history = crate::operations::archive::history::ArchiveHistory::new(history_directory);
    match confirmed.request().connection.mode {
        SyncMode::AddOnly => match execute_add_only_actions(
            confirmed.request().direction,
            confirmed.preflight().preflight().plan().actions(),
            &transfer,
            cancellation,
            observer,
            jitter_seed,
        )
        .await
        {
            Ok(result) => Ok(result),
            Err(error) => Ok(failed_action(error.error, error.result)),
        },
        SyncMode::Mirror => match execute_confirmed_mirror(
            confirmed.preflight().preflight().plan(),
            confirmed.preflight().destructive_confirmation(),
            &transfer,
            cancellation,
            observer,
            jitter_seed,
        )
        .await
        {
            Ok(result) => Ok(result),
            Err(MirrorExecutionError::Action { error, result }) => Ok(failed_action(error, result)),
            Err(_) => Err(CapabilityError::InvalidReference),
        },
        SyncMode::Archive => Ok(crate::providers::s3::operations::archive::execute(
            confirmed.request(),
            confirmed.preflight().preflight(),
            &transfer,
            cancellation,
            &history,
            history_directory,
            observer,
            jitter_seed,
        )
        .await),
    }
}

fn failed(message: impl Into<String>) -> ExecutionResult {
    ExecutionResult::failed_before_start_with_message(message)
}

struct TokioSleeper;

impl RetrySleeper for TokioSleeper {
    fn sleep(&self, delay: Duration) -> impl std::future::Future<Output = ()> + Send {
        tokio::time::sleep(delay)
    }
}

#[cfg(target_os = "windows")]
fn local_case_sensitivity() -> crate::preflight::CaseSensitivity {
    crate::preflight::CaseSensitivity::Insensitive
}

#[cfg(not(target_os = "windows"))]
fn local_case_sensitivity() -> crate::preflight::CaseSensitivity {
    crate::preflight::CaseSensitivity::Sensitive
}
