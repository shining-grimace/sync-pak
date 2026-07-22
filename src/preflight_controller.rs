use std::rc::Rc;

use slint::{ModelRc, SharedString, VecModel};

use crate::{
    AppWindow, PreflightRow,
    preflight::Preflight,
    preflight_presentation::{PreflightItemPresentation, PreflightPresentation},
};

/// Clears the review model while read-only inventory collection is in progress.
pub fn show_loading(window: &AppWindow) {
    window.set_preflight_loading(true);
    window.set_preflight_failed(false);
    window.set_preflight_items(empty_rows());
    window.set_preflight_additions(SharedString::default());
    window.set_preflight_overwrites(SharedString::default());
    window.set_preflight_deletions(SharedString::default());
    window.set_preflight_skipped(SharedString::default());
    window.set_preflight_start_action(SharedString::default());
    window.set_preflight_requires_mirror_confirmation(false);
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
    window.set_preflight_items(rows(presentation.items));
    window.set_page(11);
}

/// Shows a preflight failure without preserving an earlier connection's review items.
pub fn show_failed(window: &AppWindow) {
    window.set_preflight_loading(false);
    window.set_preflight_failed(true);
    window.set_preflight_items(empty_rows());
    window.set_preflight_start_action(SharedString::default());
    window.set_page(11);
}

fn rows(items: Vec<PreflightItemPresentation>) -> ModelRc<PreflightRow> {
    ModelRc::new(Rc::new(VecModel::from_iter(items.into_iter().map(
        |item| PreflightRow {
            path: item.path.into(),
            status: item.status.into(),
        },
    ))))
}

fn empty_rows() -> ModelRc<PreflightRow> {
    ModelRc::new(Rc::new(VecModel::default()))
}
