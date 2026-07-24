use std::{cell::RefCell, rc::Rc};

use slint::{ComponentHandle, SharedString};

use crate::{
    AppWindow,
    configuration::{DiagnosticLog, StructuredError},
};

pub(crate) type SharedDiagnosticLog = Rc<RefCell<DiagnosticLog>>;

pub(crate) fn configure(window: &AppWindow, log: SharedDiagnosticLog) {
    let weak = window.as_weak();
    let show_log = Rc::clone(&log);
    window.on_show_diagnostics(move || show(&weak, &show_log));

    let weak = window.as_weak();
    window.on_dismiss_diagnostics(move || dismiss(&weak));

    let weak = window.as_weak();
    let paths_log = Rc::clone(&log);
    window.on_set_diagnostic_include_paths(move |include_paths| {
        refresh(&weak, &paths_log, include_paths);
    });
}

pub(crate) fn record(log: &SharedDiagnosticLog, error: StructuredError) {
    log.borrow_mut().record(error);
}

pub(crate) fn present(
    window: &AppWindow,
    log: &SharedDiagnosticLog,
    summary: &'static str,
    technical_details: &'static str,
    message: &'static str,
) {
    record(log, StructuredError::new(summary, technical_details));
    window.set_diagnostic_entry_count(log.borrow().report().errors.len() as i32);
    window.set_status_message(message.into());
}

fn show(weak: &slint::Weak<AppWindow>, log: &SharedDiagnosticLog) {
    refresh(weak, log, false);
    if let Some(window) = weak.upgrade() {
        if window.get_page() != 8 {
            window.set_diagnostic_return_page(window.get_page());
        }
        window.set_diagnostic_include_paths(false);
        window.set_diagnostic_copy_status(SharedString::default());
        window.set_status_message(SharedString::default());
        window.set_page(8);
    }
}

fn dismiss(weak: &slint::Weak<AppWindow>) {
    if let Some(window) = weak.upgrade() {
        window.set_status_message(SharedString::default());
        window.set_notice_message(SharedString::default());
        window.set_diagnostic_copy_status(SharedString::default());
        window.set_page(window.get_diagnostic_return_page());
    }
}

fn refresh(weak: &slint::Weak<AppWindow>, log: &SharedDiagnosticLog, include_paths: bool) {
    let Some(window) = weak.upgrade() else { return };
    let report = log.borrow().report();
    window.set_diagnostic_entry_count(report.errors.len() as i32);
    window.set_diagnostic_text(report.redacted_text(include_paths).into());
}
