use std::rc::Rc;

#[cfg(feature = "provider-s3")]
use std::cell::RefCell;

use slint::{ComponentHandle, ModelRc, SharedString, VecModel};

mod filters;
use filters::filter_rows;
#[cfg(test)]
use filters::matches_filter;

use crate::{
    AppWindow, PreflightRow,
    app::preflight::presentation::{PreflightItemPresentation, PreflightPresentation},
    preflight::Preflight,
};

#[cfg(feature = "provider-s3")]
use crate::{
    operations::request::RunRequest,
    preflight::reviewed_operation::{ConfirmedOperation, ReviewedOperation},
};

#[cfg(feature = "provider-s3")]
thread_local! {
    static REVIEWED_OPERATION: RefCell<Option<ReviewedOperation>> = const { RefCell::new(None) };
}

/// Connects filtering controls to a derived, display-only preflight row model.
pub(crate) fn configure(window: &AppWindow) {
    #[cfg(feature = "provider-s3")]
    super::mirror_controller::configure(window);
    let weak = window.as_weak();
    window.on_set_preflight_item_filter(move |filter| {
        if let Some(window) = weak.upgrade() {
            set_filter(&window, filter);
        }
    });
}

/// Clears the review model while read-only inventory collection is in progress.
pub fn show_loading(window: &AppWindow) {
    clear_reviewed_operation();
    window.set_preflight_selection_error("".into());
    window.set_preflight_generation(next_generation(window.get_preflight_generation()));
    window.set_preflight_loading(true);
    window.set_preflight_failed(false);
    window.set_preflight_failure_message(SharedString::default());
    window.set_preflight_items(empty_rows());
    window.set_preflight_additions(SharedString::default());
    window.set_preflight_overwrites(SharedString::default());
    window.set_preflight_deletions(SharedString::default());
    window.set_preflight_skipped(SharedString::default());
    window.set_preflight_unchanged(SharedString::default());
    window.set_preflight_planned_file_changes(0);
    window.set_preflight_has_planned_actions(false);
    window.set_preflight_start_action(SharedString::default());
    window.set_preflight_requires_mirror_confirmation(false);
    window.set_preflight_mirror_confirmed(false);
    reset_filter(window);
    window.set_page(11);
}

/// Shows a completed, immutable preflight review without exposing credentials.
pub fn show_review(window: &AppWindow, preflight: &Preflight) {
    if preflight.plan().mode() == crate::configuration::SyncMode::AddOnly {
        super::reconciliation::present(
            window,
            &crate::preflight::mirror_selection::MirrorSelection::new(preflight.clone()),
            false,
        );
    }
    let presentation = PreflightPresentation::from(preflight);
    window.set_preflight_loading(false);
    window.set_preflight_failed(false);
    window.set_preflight_failure_message(SharedString::default());
    window.set_preflight_additions(presentation.additions.into());
    window.set_preflight_overwrites(presentation.overwrites.into());
    window.set_preflight_deletions(presentation.deletions.into());
    window.set_preflight_skipped(presentation.skipped.into());
    window.set_preflight_unchanged(presentation.unchanged.into());
    window.set_preflight_planned_file_changes(presentation.planned_file_changes as i32);
    window.set_preflight_has_planned_actions(presentation.has_planned_actions);
    window.set_preflight_start_action(presentation.start_action.into());
    window.set_preflight_requires_mirror_confirmation(presentation.requires_mirror_confirmation);
    window.set_preflight_mirror_confirmed(false);
    window.set_preflight_items(rows(presentation.items));
    reset_filter(window);
    window.set_page(11);
}

/// Shows and retains the immutable review that may later be confirmed for queue submission.
#[cfg(feature = "provider-s3")]
pub(crate) fn show_reviewed(window: &AppWindow, request: RunRequest, preflight: &Preflight) {
    if request.connection.mode != crate::configuration::SyncMode::Archive {
        super::mirror_controller::begin(window, request, preflight);
        return;
    }
    set_reviewed(Some(ReviewedOperation::new(request, preflight.clone())));
    show_review(window, preflight);
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
    window.set_preflight_items(empty_rows());
    window.set_preflight_additions(SharedString::default());
    window.set_preflight_overwrites(SharedString::default());
    window.set_preflight_deletions(SharedString::default());
    window.set_preflight_skipped(SharedString::default());
    window.set_preflight_unchanged(SharedString::default());
    window.set_preflight_planned_file_changes(0);
    window.set_preflight_has_planned_actions(false);
    window.set_preflight_start_action(SharedString::default());
    window.set_preflight_requires_mirror_confirmation(false);
    window.set_preflight_mirror_confirmed(false);
    reset_filter(window);
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
    super::mirror_controller::clear();
    #[cfg(feature = "provider-s3")]
    REVIEWED_OPERATION.with(|review| *review.borrow_mut() = None);
}

#[cfg_attr(not(feature = "provider-s3"), allow(dead_code))]
pub(crate) fn is_current_generation(expected: i32, current: i32) -> bool {
    expected == current
}

fn next_generation(current: i32) -> i32 {
    current.wrapping_add(1)
}

fn rows(items: Vec<PreflightItemPresentation>) -> ModelRc<PreflightRow> {
    ModelRc::new(Rc::new(VecModel::from_iter(items.into_iter().map(
        |item| PreflightRow {
            path: item.path.into(),
            status: item.status.into(),
            detail: item.detail.into(),
        },
    ))))
}

fn empty_rows() -> ModelRc<PreflightRow> {
    ModelRc::new(Rc::new(VecModel::default()))
}

fn reset_filter(window: &AppWindow) {
    set_filter(window, 0);
}

fn set_filter(window: &AppWindow, filter: i32) {
    let filter = filter.clamp(0, 4);
    window.set_preflight_item_filter(filter);
    window.set_preflight_visible_items(filter_rows(&window.get_preflight_items(), filter));
}

#[cfg(test)]
mod tests;
