use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};

use crate::{
    AppWindow, PreflightRow,
    preflight::Preflight,
    preflight_presentation::{PreflightItemPresentation, PreflightPresentation},
};

/// Clears the review model while read-only inventory collection is in progress.
pub fn show_loading(window: &AppWindow) {
    window.set_preflight_generation(next_generation(window.get_preflight_generation()));
    window.set_preflight_loading(true);
    window.set_preflight_failed(false);
    window.set_preflight_items(empty_rows());
    window.set_preflight_additions(SharedString::default());
    window.set_preflight_overwrites(SharedString::default());
    window.set_preflight_deletions(SharedString::default());
    window.set_preflight_skipped(SharedString::default());
    window.set_preflight_start_action(SharedString::default());
    window.set_preflight_requires_mirror_confirmation(false);
    window.set_preflight_mirror_confirmed(false);
    window.set_page(11);
}

/// Shows a completed, immutable preflight review without exposing credentials.
pub fn show_review(window: &AppWindow, preflight: &Preflight) {
    let presentation = PreflightPresentation::from(preflight);
    window.set_preflight_loading(false);
    window.set_preflight_failed(false);
    window.set_preflight_additions(presentation.additions.into());
    window.set_preflight_overwrites(presentation.overwrites.into());
    window.set_preflight_deletions(presentation.deletions.into());
    window.set_preflight_skipped(presentation.skipped.into());
    window.set_preflight_start_action(presentation.start_action.into());
    window.set_preflight_requires_mirror_confirmation(presentation.requires_mirror_confirmation);
    window.set_preflight_mirror_confirmed(false);
    window.set_preflight_items(rows(presentation.items));
    window.set_page(11);
}

/// Shows a preflight failure without preserving an earlier connection's review items.
pub fn show_failed(window: &AppWindow) {
    window.set_preflight_loading(false);
    window.set_preflight_failed(true);
    window.set_preflight_items(empty_rows());
    window.set_preflight_additions(SharedString::default());
    window.set_preflight_overwrites(SharedString::default());
    window.set_preflight_deletions(SharedString::default());
    window.set_preflight_skipped(SharedString::default());
    window.set_preflight_start_action(SharedString::default());
    window.set_preflight_requires_mirror_confirmation(false);
    window.set_preflight_mirror_confirmed(false);
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

#[cfg(test)]
mod tests {
    use super::{is_current_generation, next_generation};

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
}
