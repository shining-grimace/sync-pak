use std::rc::Rc;

use slint::{ComponentHandle, Model, ModelRc, SharedString, VecModel};

use crate::{
    AppWindow, PreflightRow,
    preflight::Preflight,
    preflight_presentation::{PreflightItemPresentation, PreflightPresentation},
};

/// Connects filtering controls to a derived, display-only preflight row model.
pub(crate) fn configure(window: &AppWindow) {
    let weak = window.as_weak();
    window.on_set_preflight_item_filter(move |filter| {
        if let Some(window) = weak.upgrade() {
            set_filter(&window, filter);
        }
    });
}

/// Clears the review model while read-only inventory collection is in progress.
pub fn show_loading(window: &AppWindow) {
    window.set_preflight_generation(next_generation(window.get_preflight_generation()));
    window.set_preflight_loading(true);
    window.set_preflight_failed(false);
    window.set_preflight_failure_message(SharedString::default());
    window.set_preflight_items(empty_rows());
    window.set_preflight_additions(SharedString::default());
    window.set_preflight_overwrites(SharedString::default());
    window.set_preflight_deletions(SharedString::default());
    window.set_preflight_skipped(SharedString::default());
    window.set_preflight_start_action(SharedString::default());
    window.set_preflight_requires_mirror_confirmation(false);
    window.set_preflight_mirror_confirmed(false);
    reset_filter(window);
    window.set_page(11);
}

/// Shows a completed, immutable preflight review without exposing credentials.
pub fn show_review(window: &AppWindow, preflight: &Preflight) {
    let presentation = PreflightPresentation::from(preflight);
    window.set_preflight_loading(false);
    window.set_preflight_failed(false);
    window.set_preflight_failure_message(SharedString::default());
    window.set_preflight_additions(presentation.additions.into());
    window.set_preflight_overwrites(presentation.overwrites.into());
    window.set_preflight_deletions(presentation.deletions.into());
    window.set_preflight_skipped(presentation.skipped.into());
    window.set_preflight_start_action(presentation.start_action.into());
    window.set_preflight_requires_mirror_confirmation(presentation.requires_mirror_confirmation);
    window.set_preflight_mirror_confirmed(false);
    window.set_preflight_items(rows(presentation.items));
    reset_filter(window);
    window.set_page(11);
}

/// Shows a preflight failure without preserving an earlier connection's review items.
pub fn show_failed(window: &AppWindow, message: &str) {
    window.set_preflight_loading(false);
    window.set_preflight_failed(true);
    window.set_preflight_failure_message(message.into());
    window.set_preflight_items(empty_rows());
    window.set_preflight_additions(SharedString::default());
    window.set_preflight_overwrites(SharedString::default());
    window.set_preflight_deletions(SharedString::default());
    window.set_preflight_skipped(SharedString::default());
    window.set_preflight_start_action(SharedString::default());
    window.set_preflight_requires_mirror_confirmation(false);
    window.set_preflight_mirror_confirmed(false);
    reset_filter(window);
    window.set_page(11);
}

/// Retires any in-flight inventory result when the person leaves its review.
pub fn invalidate(window: &AppWindow) {
    window.set_preflight_generation(next_generation(window.get_preflight_generation()));
    window.set_preflight_loading(false);
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

fn filter_rows(rows: &ModelRc<PreflightRow>, filter: i32) -> ModelRc<PreflightRow> {
    ModelRc::new(Rc::new(VecModel::from_iter(
        rows.iter()
            .filter(|item| matches_filter(item.status.as_str(), filter)),
    )))
}

fn matches_filter(status: &str, filter: i32) -> bool {
    match filter {
        0 => true,
        1 => status == "New",
        2 => status == "Will overwrite",
        3 => status == "Will delete",
        4 => status == "Warning",
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::{is_current_generation, matches_filter, next_generation};

    #[test]
    fn only_the_current_preflight_attempt_can_publish_a_result() {
        assert!(is_current_generation(4, 4));
        assert!(!is_current_generation(4, 5));
    }

    #[test]
    fn generation_advances_across_integer_wraparound() {
        assert_eq!(next_generation(4), 5);
        assert_eq!(next_generation(i32::MAX), i32::MIN);
    }

    #[test]
    fn planned_rows_are_categorised_by_the_selected_filter() {
        assert!(matches_filter("New", 1));
        assert!(matches_filter("Will overwrite", 2));
        assert!(matches_filter("Will delete", 3));
        assert!(matches_filter("Warning", 4));
        assert!(matches_filter("Unchanged", 0));
        assert!(!matches_filter("Warning", 3));
        assert!(!matches_filter("New", 8));
    }
}
