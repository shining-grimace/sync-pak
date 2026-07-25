use std::{collections::HashMap, path::PathBuf, sync::Mutex, time::Duration};

use crate::{
    add_only_execution::execute_add_only_actions,
    cancellation::CancellationToken,
    capabilities::CapabilityError,
    configuration::{ConfigStore, ProviderRepository, SyncMode},
    execution::{ExecutionResult, OperationExecutor},
    local_remote_transfer::LocalRemoteTransfer,
    mirror_execution::execute_confirmed_mirror,
    platform::PlatformCredentialStore,
    queue::QueueEntry,
    retry::{RetryPolicy, RetrySleeper},
    s3_preflight::{S3PreflightError, collect_s3_connection_preflight},
    s3_transport::S3Transport,
    transfer_paths::{LocalTransferRoot, RemoteTransferPrefix},
    transfer_progress::TransferProgressObserver,
};

/// Runs confirmed S3 operations using the configuration and protected credentials for this launch.
pub struct S3OperationExecutor {
    configuration_path: PathBuf,
    cancellations: Mutex<HashMap<String, CancellationToken>>,
}

impl S3OperationExecutor {
    pub fn new(configuration_path: PathBuf) -> Self {
        Self {
            configuration_path,
            cancellations: Mutex::new(HashMap::new()),
        }
    }

    fn cancellation_for(&self, connection_id: &str) -> CancellationToken {
        let token = CancellationToken::default();
        self.cancellations
            .lock()
            .expect("cancellation mutex poisoned")
            .insert(connection_id.into(), token.clone());
        token
    }

    fn clear_cancellation(&self, connection_id: &str) {
        self.cancellations
            .lock()
            .expect("cancellation mutex poisoned")
            .remove(connection_id);
    }
}

impl OperationExecutor for S3OperationExecutor {
    fn execute(
        &self,
        entry: &QueueEntry,
        observer: &dyn TransferProgressObserver,
    ) -> Result<ExecutionResult, CapabilityError> {
        let confirmed = entry
            .confirmed_operation
            .as_ref()
            .ok_or(CapabilityError::InvalidReference)?;
        let connection_id = confirmed.request().connection.id.as_str().to_owned();
        let cancellation = self.cancellation_for(&connection_id);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| CapabilityError::Unavailable)?;
        let result = runtime.block_on(execute_confirmed(
            confirmed,
            &self.configuration_path,
            &cancellation,
            observer,
            entry.operation_id.as_u128() as u64,
        ));
        self.clear_cancellation(&connection_id);
        result
    }

    fn cancel(&self, connection_id: &str) -> Result<(), CapabilityError> {
        let token = self
            .cancellations
            .lock()
            .expect("cancellation mutex poisoned")
            .get(connection_id)
            .cloned()
            .ok_or(CapabilityError::NotFound)?;
        token.cancel();
        Ok(())
    }
}

async fn execute_confirmed(
    confirmed: &crate::reviewed_operation::ConfirmedOperation,
    configuration_path: &std::path::Path,
    cancellation: &CancellationToken,
    observer: &dyn TransferProgressObserver,
    jitter_seed: u64,
) -> Result<ExecutionResult, CapabilityError> {
    let configuration = ConfigStore::at(configuration_path.into());
    let credentials = PlatformCredentialStore::new()?;
    let providers = ProviderRepository::new(&configuration, &credentials);
    let current = match collect_s3_connection_preflight(
        confirmed.request(),
        &providers,
        local_case_sensitivity(),
    )
    .await
    {
        Ok(current) => current,
        Err(S3PreflightError::Credentials(_)) => {
            return Ok(failed(
                "SyncPak could not access the saved credentials. Unlock protected storage, then try again.",
            ));
        }
        Err(S3PreflightError::Provider(_)) => {
            return Ok(failed(
                "SyncPak could not reach this provider. Check its credentials, bucket, and network connection.",
            ));
        }
        Err(S3PreflightError::Inventory(_)) => {
            return Ok(failed(
                "SyncPak could not recheck the source and destination. Refresh the review and try again.",
            ));
        }
    };
    if current != *confirmed.preflight().preflight() {
        return Ok(failed(
            "Files changed since this review. Refresh the review before starting the operation.",
        ));
    }
    let provider_credentials = match providers.load_credentials(&confirmed.request().provider.id) {
        Ok(credentials) => credentials,
        Err(_) => {
            return Ok(failed(
                "SyncPak could not access the saved credentials. Unlock protected storage, then try again.",
            ));
        }
    };
    let transport = match S3Transport::connect(&confirmed.request().provider, provider_credentials)
        .await
    {
        Ok(transport) => transport,
        Err(_) => {
            return Ok(failed(
                "SyncPak could not connect to this provider. Check its settings and network connection.",
            ));
        }
    };
    let retry = RetryPolicy::default();
    let sleeper = TokioSleeper;
    let transfer = LocalRemoteTransfer::new(
        &transport,
        &confirmed.request().connection.bucket,
        LocalTransferRoot::new(&confirmed.request().connection.local_path),
        RemoteTransferPrefix::new(&confirmed.request().connection.remote_path)
            .map_err(|_| CapabilityError::InvalidReference)?,
        &retry,
        &sleeper,
    );
    let history_directory = configuration_path
        .parent()
        .ok_or(CapabilityError::Unavailable)?;
    let history = crate::archive_history::ArchiveHistory::new(history_directory);
    match confirmed.request().connection.mode {
        SyncMode::AddOnly => execute_add_only_actions(
            confirmed.request().direction,
            confirmed.preflight().preflight().plan().actions(),
            &transfer,
            cancellation,
            observer,
            jitter_seed,
        )
        .await
        .map_err(|_| CapabilityError::Unexpected),
        SyncMode::Mirror => execute_confirmed_mirror(
            confirmed.preflight().preflight().plan(),
            confirmed.preflight().destructive_confirmation(),
            &transfer,
            cancellation,
            observer,
            jitter_seed,
        )
        .await
        .map_err(|_| CapabilityError::Unexpected),
        SyncMode::Archive => Ok(crate::s3_archive_operation::execute(
            confirmed.request(),
            confirmed.preflight().preflight(),
            &transfer,
            cancellation,
            &history,
            observer,
            jitter_seed,
        )
        .await),
    }
}

fn failed(message: &'static str) -> ExecutionResult {
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

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::{capabilities::CapabilityError, execution::OperationExecutor};

    use super::S3OperationExecutor;

    #[test]
    fn cancellation_requires_an_active_connection() {
        let executor = S3OperationExecutor::new(PathBuf::from("config.json"));

        assert_eq!(executor.cancel("missing"), Err(CapabilityError::NotFound));
    }
}
