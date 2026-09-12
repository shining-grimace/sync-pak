use super::{
    files, presentation,
    review::{Review, State},
};
use crate::{
    AppWindow, SettingsState,
    configuration::{ConfigStore, lists::export_list},
};
use slint::ComponentHandle;

pub(super) fn open(window: &AppWindow, state: &State, mode: i32) {
    let result = (|| {
        let config = ConfigStore::for_current_platform()
            .and_then(|s| s.load())
            .map_err(|e| e.to_string())?;
        Ok::<_, String>(Review::new(config, None, mode))
    })();
    match result {
        Ok(review) => {
            *state.borrow_mut() = Some(review);
            window
                .global::<SettingsState>()
                .set_error(Default::default());
            presentation::review(window, state);
        }
        Err(error) => window.global::<SettingsState>().set_error(error.into()),
    }
}

pub(super) fn confirm(window: &AppWindow, state: &State) {
    if window.global::<SettingsState>().get_busy() {
        return;
    }
    let borrowed = state.borrow();
    let Some(review) = borrowed.as_ref() else {
        return;
    };
    let editing_root = review.mode == 3;
    let result = if review.mode == 2 {
        let selected = review
            .original
            .connections
            .iter()
            .filter(|c| review.selected.contains(c.id.as_str()))
            .map(|c| c.id.clone())
            .collect::<Vec<_>>();
        match export_list(&review.original, &selected) {
            Ok(bytes) => {
                files::export(window, state.clone(), bytes);
                return;
            }
            Err(error) => Err(error),
        }
    } else {
        ConfigStore::for_current_platform()
            .map_err(|e| e.to_string())
            .and_then(|s| review.commit(&s))
    };
    drop(borrowed);
    match result {
        Ok(count) => {
            if !editing_root {
                window.invoke_invalidate_connection_verifications();
            }
            state.borrow_mut().take();
            window.global::<SettingsState>().set_mode(0);
            window
                .global::<SettingsState>()
                .set_error(Default::default());
            window
                .global::<SettingsState>()
                .set_notice(if editing_root {
                    "Local Root saved.".into()
                } else {
                    format!("Saved {count} new or changed connections. Verify them before syncing.")
                        .into()
                });
            presentation::refresh(window);
        }
        Err(error) => window.global::<SettingsState>().set_error(error.into()),
    }
}
