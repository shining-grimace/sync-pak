use std::{
    path::PathBuf,
    sync::mpsc::{self, Receiver},
    time::Duration,
};

use crate::{
    AppWindow,
    configuration::{ConfigStore, ProviderRepository},
    diagnostics_controller::{self, SharedDiagnosticLog},
    platform::PlatformCredentialStore,
    preflight::{CaseSensitivity, Preflight},
    run_request::RunRequest,
    s3_preflight::{S3PreflightError, collect_s3_connection_preflight},
};

/// Runs S3 inventory collection away from the UI event loop and returns its read-only result.
pub(crate) fn start(
    weak: slint::Weak<AppWindow>,
    request: RunRequest,
    configuration_path: PathBuf,
    diagnostics: SharedDiagnosticLog,
) {
    let connection_id = request.connection.id.as_str().to_owned();
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let _ = sender.send(collect(request, configuration_path));
    });
    await_result(weak, connection_id, receiver, diagnostics);
}

fn collect(
    request: RunRequest,
    configuration_path: PathBuf,
) -> Result<Preflight, PreflightFailure> {
    let configuration = ConfigStore::at(configuration_path);
    let credentials = PlatformCredentialStore::new().map_err(|_| PreflightFailure::Credentials)?;
    let providers = ProviderRepository::new(&configuration, &credentials);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| PreflightFailure::Inventory)?;
    runtime
        .block_on(collect_s3_connection_preflight(
            &request,
            &providers,
            local_case_sensitivity(),
        ))
        .map_err(PreflightFailure::from)
}

fn await_result(
    weak: slint::Weak<AppWindow>,
    connection_id: String,
    receiver: Receiver<Result<Preflight, PreflightFailure>>,
    diagnostics: SharedDiagnosticLog,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        let Some(window) = weak.upgrade() else { return };
        if !is_active(&window, &connection_id) {
            return;
        }
        match receiver.try_recv() {
            Ok(Ok(preflight)) => {
                crate::preflight_controller::show_review(&window, &preflight);
            }
            Ok(Err(failure)) => {
                crate::preflight_controller::show_failed(&window);
                diagnostics_controller::present(
                    &window,
                    &diagnostics,
                    "This operation cannot start",
                    failure.diagnostic(),
                    failure.message(),
                );
            }
            Err(mpsc::TryRecvError::Disconnected) => {
                crate::preflight_controller::show_failed(&window);
                diagnostics_controller::present(
                    &window,
                    &diagnostics,
                    "This operation cannot start",
                    "preflight worker stopped",
                    "SyncPak could not complete the preflight. Run the connection again.",
                );
            }
            Err(mpsc::TryRecvError::Empty) => {
                await_result(weak, connection_id, receiver, diagnostics)
            }
        }
    });
}

fn is_active(window: &AppWindow, connection_id: &str) -> bool {
    window.get_page() == 11
        && window.get_preflight_loading()
        && window.get_run_connection_id().as_str() == connection_id
}

#[derive(Clone, Copy)]
enum PreflightFailure {
    Credentials,
    Provider,
    Inventory,
}

impl From<S3PreflightError> for PreflightFailure {
    fn from(error: S3PreflightError) -> Self {
        match error {
            S3PreflightError::Credentials(_) => Self::Credentials,
            S3PreflightError::Provider(_) => Self::Provider,
            S3PreflightError::Inventory(_) => Self::Inventory,
        }
    }
}

impl PreflightFailure {
    fn diagnostic(self) -> &'static str {
        match self {
            Self::Credentials => "saved credential access failed",
            Self::Provider => "provider inventory failed",
            Self::Inventory => "local or remote inventory failed",
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
            Self::Inventory => {
                "SyncPak could not inventory this connection. Check the local folder and bucket, then try again."
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
