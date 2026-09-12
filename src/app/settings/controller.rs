use super::{
    actions::{confirm, open},
    files, presentation,
    review::{Review, State},
};
use crate::{AppWindow, SettingsState, configuration::lists::ConflictChoice};
use slint::ComponentHandle;

pub(crate) fn configure(window: &AppWindow) {
    let review: State = Default::default();
    let state = window.global::<SettingsState>();
    let weak = window.as_weak();
    state.on_import_file({
        let review = review.clone();
        move || {
            if let Some(window) = weak.upgrade() {
                files::import(&window, review.clone());
            }
        }
    });
    let weak = window.as_weak();
    state.on_export_list({
        let review = review.clone();
        move || {
            if let Some(window) = weak.upgrade() {
                open(&window, &review, 2);
            }
        }
    });
    let weak = window.as_weak();
    state.on_edit_roots({
        let review = review.clone();
        move || {
            if let Some(window) = weak.upgrade() {
                open(&window, &review, 3);
            }
        }
    });
    let weak = window.as_weak();
    state.on_cancel({
        let review = review.clone();
        move || {
            if let Some(window) = weak.upgrade() {
                if window.global::<SettingsState>().get_busy() {
                    return;
                }
                review.borrow_mut().take();
                window.global::<SettingsState>().set_mode(0);
                window
                    .global::<SettingsState>()
                    .set_error(Default::default());
                presentation::refresh(&window);
            }
        }
    });
    let weak = window.as_weak();
    state.on_confirm({
        let review = review.clone();
        move || {
            if let Some(window) = weak.upgrade() {
                confirm(&window, &review);
            }
        }
    });
    let weak = window.as_weak();
    state.on_pick_local({
        let review = review.clone();
        move || {
            if let Some(window) = weak.upgrade() {
                files::folder(&window, review.clone());
            }
        }
    });
    let weak = window.as_weak();
    state.on_set_local({
        let review = review.clone();
        move |path| {
            change(&weak, &review, |r| r.local_root = path.to_string());
        }
    });
    let weak = window.as_weak();
    state.on_select_connection({
        let review = review.clone();
        move |index, selected| {
            change(&weak, &review, |r| {
                if let Some(id) = r.connection_id(index as usize) {
                    if selected {
                        r.selected.insert(id);
                    } else {
                        r.selected.remove(&id);
                    }
                }
            });
        }
    });
    let weak = window.as_weak();
    state.on_set_conflict({
        let review = review.clone();
        move |index, choice| {
            change(&weak, &review, |r| {
                if let Some(id) = r.connection_id(index as usize) {
                    r.choices.insert(
                        id,
                        match choice {
                            1 => ConflictChoice::Replace,
                            2 => ConflictChoice::Copy,
                            _ => ConflictChoice::Skip,
                        },
                    );
                }
            });
        }
    });
    let weak = window.as_weak();
    state.on_set_provider(move |index, provider| {
        change(&weak, &review, |r| {
            if let Some(id) = r.connection_id(index as usize) {
                if let Some(p) = provider
                    .checked_sub(1)
                    .and_then(|i| r.original.providers.get(i as usize))
                {
                    r.mappings.insert(id, p.id.clone());
                } else {
                    r.mappings.remove(&id);
                }
            }
        });
    });
}
fn change(weak: &slint::Weak<AppWindow>, state: &State, action: impl FnOnce(&mut Review)) {
    let Some(window) = weak.upgrade() else { return };
    if window.global::<SettingsState>().get_busy() {
        return;
    }
    if let Some(review) = state.borrow_mut().as_mut() {
        action(review);
    }
    window
        .global::<SettingsState>()
        .set_error(Default::default());
    presentation::review(&window, state);
}
