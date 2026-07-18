use std::{
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

use slint::ComponentHandle;

use crate::{
    AppWindow,
    capabilities::{CapabilityError, FolderPicker, FolderSelection},
    diagnostics_controller::{self, SharedDiagnosticLog},
    platform::PlatformFolderPicker,
};

type PickResult = Arc<Mutex<Option<Result<Option<FolderSelection>, CapabilityError>>>>;

pub(crate) fn configure(window: &AppWindow, diagnostics: SharedDiagnosticLog) {
    let weak = window.as_weak();
    window.on_select_local_folder(move || select_folder(&weak, &diagnostics));
}

fn select_folder(weak: &slint::Weak<AppWindow>, diagnostics: &SharedDiagnosticLog) {
    let result: PickResult = Arc::new(Mutex::new(None));
    let completion_result = Arc::clone(&result);
    let completion = Box::new(move |selection| {
        if let Ok(mut pending) = completion_result.lock() {
            *pending = Some(selection);
        }
    });
    if let Err(error) = PlatformFolderPicker.pick_folder(completion) {
        set_error(weak, diagnostics, error);
        return;
    }
    poll_for_selection(weak.clone(), Rc::clone(diagnostics), result);
}

fn poll_for_selection(
    weak: slint::Weak<AppWindow>,
    diagnostics: SharedDiagnosticLog,
    result: PickResult,
) {
    let selection = result.lock().ok().and_then(|mut pending| pending.take());
    match selection {
        Some(Ok(Some(selection))) => match selection.display_value() {
            Ok(path) => {
                if let Some(window) = weak.upgrade() {
                    window.set_connection_form_local(path.into());
                    window.set_status_message(Default::default());
                }
            }
            Err(error) => set_error(&weak, &diagnostics, error),
        },
        Some(Ok(None)) => {}
        Some(Err(error)) => set_error(&weak, &diagnostics, error),
        None => slint::Timer::single_shot(Duration::from_millis(25), move || {
            poll_for_selection(weak, diagnostics, result)
        }),
    }
}

fn set_error(
    weak: &slint::Weak<AppWindow>,
    diagnostics: &SharedDiagnosticLog,
    error: CapabilityError,
) {
    if let Some(window) = weak.upgrade() {
        let (technical_details, message) = match error {
            CapabilityError::UnsupportedPath => (
                "selected path is not UTF-8",
                "The selected folder cannot be represented safely as UTF-8.",
            ),
            CapabilityError::Unavailable => (
                "folder picker unavailable",
                "Folder selection is unavailable right now. Enter the path manually.",
            ),
            _ => (
                "folder picker failed",
                "SyncPak could not select that folder. Try again or enter its path manually.",
            ),
        };
        diagnostics_controller::present(
            &window,
            diagnostics,
            "Folder selection could not be completed",
            technical_details,
            message,
        );
    }
}
