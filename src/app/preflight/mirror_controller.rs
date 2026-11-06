use super::{controller, reconciliation};
use crate::{
    AppWindow,
    operations::request::RunRequest,
    preflight::{
        Preflight,
        mirror_selection::{MirrorChoice, MirrorSelection},
        reviewed_operation::ReviewedOperation,
    },
};
use slint::ComponentHandle;
use std::cell::RefCell;

thread_local! {
    static SELECTION: RefCell<Option<(RunRequest, MirrorSelection)>> = const { RefCell::new(None) };
}

pub(crate) fn configure(window: &AppWindow) {
    let weak = window.as_weak();
    window.on_choose_mirror_file(move |path, section, action| {
        let Some(window) = weak.upgrade() else { return };
        SELECTION.with(|state| {
            let mut state = state.borrow_mut();
            let Some((request, selection)) = state.as_mut() else {
                return;
            };
            let choice = match (section, action) {
                (_, 0) => None,
                (0, 1) => Some(MirrorChoice::Keep),
                (0, 2) => Some(MirrorChoice::Delete),
                (1, 1) => Some(MirrorChoice::UseLocal),
                (1, 2) => Some(MirrorChoice::UseRemote),
                _ => return,
            };
            let paths = selection
                .original()
                .comparison()
                .iter()
                .filter(|entry| {
                    reconciliation::section(entry) == section
                        && (path.is_empty() || entry.path.as_str() == path.as_str())
                })
                .map(|entry| entry.path.clone())
                .collect::<Vec<_>>();
            for path in paths {
                selection.choose(&path, choice);
            }
            publish(&window, request, selection);
        });
    });
}

pub(crate) fn begin(window: &AppWindow, request: RunRequest, preflight: &Preflight) {
    let selection = MirrorSelection::new(preflight.clone());
    publish(window, &request, &selection);
    SELECTION.with(|state| *state.borrow_mut() = Some((request, selection)));
}

pub(crate) fn clear() {
    SELECTION.with(|state| *state.borrow_mut() = None);
}

fn publish(window: &AppWindow, request: &RunRequest, selection: &MirrorSelection) {
    controller::show_review(window, &selection.preview());
    match selection.selected() {
        Ok(selected) => {
            controller::set_reviewed(Some(ReviewedOperation::new(request.clone(), selected)));
            window.set_preflight_selection_error("".into());
        }
        Err(message) => {
            controller::set_reviewed(None);
            window.set_preflight_has_planned_actions(false);
            window.set_preflight_mirror_confirmed(false);
            window.set_preflight_selection_error(message.into());
        }
    }
    reconciliation::present(window, selection, true);
}
