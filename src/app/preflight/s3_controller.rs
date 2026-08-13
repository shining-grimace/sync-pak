use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use crate::{
    AppWindow,
    app::diagnostics::{self as diagnostics_controller, SharedDiagnosticLog},
    configuration::{ConfigStore, ProviderRepository},
    inventory::endpoint::{EndpointInventoryError, EndpointPreflightError},
    operations::request::RunRequest,
    platform::PlatformCredentialStore,
    preflight::{CaseSensitivity, Preflight},
    providers::s3::preflight::{S3PreflightError, collect_s3_connection_preflight},
};

/// Runs S3 inventory collection away from the UI event loop and returns its read-only result.
pub(crate) fn start(
    weak: slint::Weak<AppWindow>,
    request: RunRequest,
    configuration_path: PathBuf,
    diagnostics: SharedDiagnosticLog,
    generation: i32,
) {
    let connection_id = request.connection.id.as_str().to_owned();
    let review_request = request.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let _ = sender.send(collect(request, configuration_path));
    });
    await_result(
        weak,
        connection_id,
        generation,
        receiver,
        diagnostics,
        review_request,
    );
}

fn collect(
    request: RunRequest,
    configuration_path: PathBuf,
) -> Result<Preflight, PreflightFailure> {
    let configuration = ConfigStore::at(configuration_path.clone());
    let credentials = PlatformCredentialStore::new().map_err(|_| PreflightFailure::Credentials)?;
    let providers = ProviderRepository::new(&configuration, &credentials);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| PreflightFailure::Planning)?;
    runtime
        .block_on(collect_s3_connection_preflight(
            &request,
            &providers,
            local_case_sensitivity(),
            &configuration_path,
        ))
        .map_err(PreflightFailure::from)
}

fn await_result(
    weak: slint::Weak<AppWindow>,
    connection_id: String,
    generation: i32,
    receiver: Receiver<Result<Preflight, PreflightFailure>>,
    diagnostics: SharedDiagnosticLog,
    review_request: RunRequest,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(window) = weak.upgrade() else { return };
        if !is_active(&window, &connection_id, generation) {
            return;
        }
        match receiver.try_recv() {
            Ok(Ok(preflight)) => {
                crate::app::preflight::controller::show_reviewed(
                    &window,
                    review_request,
                    &preflight,
                );
            }
            Ok(Err(failure)) => {
                crate::app::preflight::controller::show_failed(&window, failure.message());
                diagnostics_controller::present(
                    &window,
                    &diagnostics,
                    "This operation cannot start",
                    failure.diagnostic(),
                    failure.message(),
                );
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                crate::app::preflight::controller::show_failed(
                    &window,
                    "SyncPak could not complete the preflight. Run the connection again.",
                );
                diagnostics_controller::present(
                    &window,
                    &diagnostics,
                    "This operation cannot start",
                    "preflight worker stopped",
                    "SyncPak could not complete the preflight. Run the connection again.",
                );
            }
            Err(mpsc::TryRecvError::Empty) => await_result(
                weak,
                connection_id,
                generation,
                receiver,
                diagnostics,
                review_request,
            ),
        }
    });
}

fn is_active(window: &AppWindow, connection_id: &str, generation: i32) -> bool {
    window.get_page() == 11
        && window.get_preflight_loading()
        && window.get_run_connection_id().as_str() == connection_id
        && crate::app::preflight::controller::is_current_generation(
            generation,
            window.get_preflight_generation(),
        )
}

#[derive(Clone, Copy)]
enum PreflightFailure {
    Credentials,
    Provider,
    LocalInventory,
    RemoteInventory,
    Planning,
}

impl From<S3PreflightError> for PreflightFailure {
    fn from(error: S3PreflightError) -> Self {
        match error {
            S3PreflightError::Credentials(_) => Self::Credentials,
            S3PreflightError::Provider(_) => Self::Provider,
            S3PreflightError::Inventory(
                EndpointPreflightError::SourceInventory(EndpointInventoryError::Local(_))
                | EndpointPreflightError::DestinationInventory(EndpointInventoryError::Local(_)),
            ) => Self::LocalInventory,
            S3PreflightError::Inventory(
                EndpointPreflightError::SourceInventory(EndpointInventoryError::Remote(_))
                | EndpointPreflightError::DestinationInventory(EndpointInventoryError::Remote(_)),
            ) => Self::RemoteInventory,
            S3PreflightError::Inventory(EndpointPreflightError::Preflight(_)) => Self::Planning,
        }
    }
}

impl PreflightFailure {
    fn diagnostic(self) -> &'static str {
        match self {
            Self::Credentials => "saved credential access failed",
            Self::Provider => "provider inventory failed",
            Self::LocalInventory => "local folder inventory failed",
            Self::RemoteInventory => "cloud path inventory failed",
            Self::Planning => "connection preflight planning failed",
        }
    }
    fn message(self) -> &'static str {
        match self {
            Self::Credentials => {
                "SyncPak could not access the saved credentials. Unlock protected storage, then try again."
            }
            Self::Provider => {
                "SyncPak could not reach this provider. Check its credentials, bucket, and network connection."
            }
            Self::LocalInventory => {
                "SyncPak could not read the configured local folder. Check that it exists and that SyncPak has permission to access it."
            }
            Self::RemoteInventory => {
                "SyncPak could not read the configured cloud bucket or remote folder. Check that the cloud path exists and that the provider credentials can access it."
            }
            Self::Planning => {
                "SyncPak read both paths but could not prepare this operation. Review the connection settings, then try again."
            }
        }
    }
}

#[cfg(target_os = "windows")]
fn local_case_sensitivity() -> CaseSensitivity {
    CaseSensitivity::Insensitive
}

#[cfg(not(target_os = "windows"))]
fn local_case_sensitivity() -> CaseSensitivity {
    CaseSensitivity::Sensitive
}

#[cfg(test)]
mod tests;
