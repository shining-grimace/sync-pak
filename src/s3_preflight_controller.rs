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
    s3_preflight::collect_s3_connection_preflight,
};

/// Runs S3 inventory collection away from the UI event loop and returns its read-only result.
pub(crate) fn start(
    weak: slint::Weak<AppWindow>,
    request: RunRequest,
    configuration_path: PathBuf,
    diagnostics: SharedDiagnosticLog,
) {
    let (sender, receiver) = mpsc::sync_channel(1);
    std::thread::spawn(move || {
        let _ = sender.send(collect(request, configuration_path));
    });
    await_result(weak, receiver, diagnostics);
}

fn collect(request: RunRequest, configuration_path: PathBuf) -> Result<Preflight, ()> {
    let configuration = ConfigStore::at(configuration_path);
    let credentials = PlatformCredentialStore::new().map_err(|_| ())?;
    let providers = ProviderRepository::new(&configuration, &credentials);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .map_err(|_| ())?;
    runtime
        .block_on(collect_s3_connection_preflight(
            &request,
            &providers,
            local_case_sensitivity(),
        ))
        .map_err(|_| ())
}

fn await_result(
    weak: slint::Weak<AppWindow>,
    receiver: Receiver<Result<Preflight, ()>>,
    diagnostics: SharedDiagnosticLog,
) {
    slint::Timer::single_shot(Duration::from_millis(50), move || {
        match receiver.try_recv() {
            Ok(Ok(preflight)) => {
                if let Some(window) = weak.upgrade() {
                    crate::preflight_controller::show_review(&window, &preflight);
                }
            }
            Ok(Err(())) | Err(mpsc::TryRecvError::Disconnected) => {
                if let Some(window) = weak.upgrade() {
                    crate::preflight_controller::show_failed(&window);
                    diagnostics_controller::present(
                        &window,
                        &diagnostics,
                        "This operation cannot start",
                        "S3 preflight collection failed",
                        "SyncPak could not list this connection. Check its provider, bucket, and local folder, then try again.",
                    );
                }
            }
            Err(mpsc::TryRecvError::Empty) => await_result(weak, receiver, diagnostics),
        }
    });
}

#[cfg(target_os = "windows")]
fn local_case_sensitivity() -> CaseSensitivity {
    CaseSensitivity::Insensitive
}

#[cfg(not(target_os = "windows"))]
fn local_case_sensitivity() -> CaseSensitivity {
    CaseSensitivity::Sensitive
}
