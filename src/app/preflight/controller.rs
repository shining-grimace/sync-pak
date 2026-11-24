use slint::SharedString;

use crate::AppWindow;

#[cfg(feature = "provider-s3")]
use std::cell::RefCell;

#[cfg(feature = "provider-s3")]
use crate::preflight::Preflight;

#[cfg(feature = "provider-s3")]
use crate::{
    operations::request::RunRequest,
    preflight::reviewed_operation::{ConfirmedOperation, ReviewedOperation},
};

#[cfg(feature = "provider-s3")]
thread_local! {
    static REVIEWED_OPERATION: RefCell<Option<ReviewedOperation>> = const { RefCell::new(None) };
}

/// Connects the interactive review controls.
pub(crate) fn configure(_window: &AppWindow) {
    #[cfg(feature = "provider-s3")]
    {
        super::mirror_controller::configure(_window);
        super::comparison::configure(_window);
        super::archive_controller::configure(_window);
    }
}

/// Clears the review model while read-only inventory collection is in progress.
pub fn show_loading(window: &AppWindow) {
    clear_reviewed_operation();
    window.set_preflight_selection_error("".into());
    window.set_preflight_archive_rows(Default::default());
    window.set_preflight_generation(next_generation(window.get_preflight_generation()));
    window.set_preflight_loading(true);
    window.set_preflight_failed(false);
    window.set_preflight_failure_message(SharedString::default());
    window.set_preflight_has_planned_actions(false);
    window.set_preflight_start_action(SharedString::default());
    window.set_preflight_requires_mirror_confirmation(false);
    window.set_preflight_mirror_confirmed(false);
    window.set_page(11);
}

/// Shows a completed read-only preflight and its available start action.
#[cfg(feature = "provider-s3")]
pub(super) fn show_review(window: &AppWindow, preflight: &Preflight) {
    let plan = preflight.plan();
    window.set_preflight_loading(false);
    window.set_preflight_failed(false);
    window.set_preflight_failure_message(SharedString::default());
    let summary = plan.summary();
    window
        .set_preflight_has_planned_actions(summary.planned_file_changes() + summary.archives() > 0);
    window.set_preflight_start_action(super::presentation::start_action(plan).into());
    window.set_preflight_requires_mirror_confirmation(plan.requires_confirmation());
    window.set_preflight_mirror_confirmed(false);
    window.set_page(11);
}

/// Shows and retains the immutable review that may later be confirmed for queue submission.
#[cfg(feature = "provider-s3")]
pub(crate) fn show_reviewed(window: &AppWindow, request: RunRequest, preflight: &Preflight) {
    if request.connection.mode == crate::configuration::SyncMode::Archive {
        super::archive_controller::begin(window, request, preflight);
    } else {
        super::mirror_controller::begin(window, request, preflight);
    }
}

/// Consumes the displayed review, binding any destructive acknowledgement to its exact plan.
#[cfg(feature = "provider-s3")]
pub(crate) fn take_confirmed(
    mirror_acknowledged: bool,
) -> Result<ConfirmedOperation, crate::preflight::confirmed::StartError> {
    REVIEWED_OPERATION.with(|review| {
        let confirmed = review
            .borrow()
            .as_ref()
            .cloned()
            .ok_or(crate::preflight::confirmed::StartError::AcknowledgementRequired)?
            .confirm(mirror_acknowledged)?;
        *review.borrow_mut() = None;
        Ok(confirmed)
    })
}

/// Shows a preflight failure without preserving an earlier connection's review items.
pub fn show_failed(window: &AppWindow, message: &str) {
    clear_reviewed_operation();
    window.set_preflight_selection_error("".into());
    window.set_preflight_loading(false);
    window.set_preflight_failed(true);
    window.set_preflight_failure_message(message.into());
    window.set_preflight_has_planned_actions(false);
    window.set_preflight_start_action(SharedString::default());
    window.set_preflight_requires_mirror_confirmation(false);
    window.set_preflight_mirror_confirmed(false);
    window.set_page(11);
}

/// Retires any in-flight inventory result when the person leaves its review.
pub fn invalidate(window: &AppWindow) {
    clear_reviewed_operation();
    window.set_preflight_selection_error("".into());
    window.set_preflight_generation(next_generation(window.get_preflight_generation()));
    window.set_preflight_loading(false);
}

#[cfg(feature = "provider-s3")]
pub(crate) fn set_reviewed(operation: Option<ReviewedOperation>) {
    REVIEWED_OPERATION.with(|review| *review.borrow_mut() = operation);
}

fn clear_reviewed_operation() {
    #[cfg(feature = "provider-s3")]
    {
        super::mirror_controller::clear();
        super::archive_controller::clear();
        REVIEWED_OPERATION.with(|review| *review.borrow_mut() = None);
    }
}

#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
pub(crate) fn is_current_generation(expected: i32, current: i32) -> bool {
    expected == current
}

fn next_generation(current: i32) -> i32 {
    current.wrapping_add(1)
}

#[cfg(test)]
mod tests;
