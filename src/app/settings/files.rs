use super::{
    presentation,
    review::{Review, State},
};
use crate::{
    AppWindow, SettingsState,
    capabilities::{
        FolderPicker,
        list_file::{ListFileCompletion, ListFilePicker},
    },
    configuration::{ConfigStore, lists::ListDocument},
    platform::{PlatformFolderPicker, list_file::PlatformListFilePicker},
};
use slint::ComponentHandle;
use std::{
    rc::Rc,
    sync::{Arc, Mutex},
    time::Duration,
};

type Pending = Arc<Mutex<Option<Result<Option<Vec<u8>>, String>>>>;

pub(super) fn import(window: &AppWindow, state: State) {
    if window.global::<SettingsState>().get_busy() {
        return;
    }
    let (pending, completion) = start(window);
    if let Err(error) = PlatformListFilePicker.open(completion) {
        fail(window, error);
        return;
    }
    poll(
        window.as_weak(),
        pending,
        Rc::new(move |window, result| {
            let Some(bytes) = result? else {
                return Ok(());
            };
            let document = ListDocument::parse(&bytes)?;
            let config = ConfigStore::for_current_platform()
                .and_then(|s| s.load())
                .map_err(|e| e.to_string())?;
            *state.borrow_mut() = Some(Review::new(config, Some(document), 1));
            presentation::review(window, &state);
            Ok(())
        }),
    );
}

pub(super) fn export(window: &AppWindow, state: State, bytes: Vec<u8>) {
    let (pending, completion) = start(window);
    if let Err(error) = PlatformListFilePicker.save(bytes, completion) {
        fail(window, error);
        return;
    }
    poll(
        window.as_weak(),
        pending,
        Rc::new(move |window, result| {
            if result?.is_some() {
                state.borrow_mut().take();
                window.global::<SettingsState>().set_mode(0);
                window
                    .global::<SettingsState>()
                    .set_notice("Connection list exported.".into());
                presentation::refresh(window);
            }
            Ok(())
        }),
    );
}

pub(super) fn folder(window: &AppWindow, state: State) {
    if window.global::<SettingsState>().get_busy() {
        return;
    }
    let (pending, completion) = start(window);
    let result = PlatformFolderPicker.pick_folder(Box::new(move |result| {
        completion(
            result
                .map_err(|_| "The folder picker could not select a folder.".into())
                .and_then(|selection| {
                    selection
                        .map(|s| {
                            s.display_value()
                                .map(|s| s.as_bytes().to_vec())
                                .map_err(|_| "Folder path is not UTF-8.".into())
                        })
                        .transpose()
                }),
        );
    }));
    if result.is_err() {
        fail(window, "The folder picker could not be opened.".into());
        return;
    }
    poll(
        window.as_weak(),
        pending,
        Rc::new(move |window, result| {
            if let Some(bytes) = result? {
                let path = String::from_utf8(bytes).map_err(|_| "Folder path is not UTF-8.")?;
                if let Some(review) = state.borrow_mut().as_mut() {
                    review.local_root = path;
                }
                presentation::review(window, &state);
            }
            Ok(())
        }),
    );
}

fn start(window: &AppWindow) -> (Pending, ListFileCompletion) {
    window.global::<SettingsState>().set_busy(true);
    window
        .global::<SettingsState>()
        .set_error(Default::default());
    let pending: Pending = Default::default();
    let result = pending.clone();
    (
        pending,
        Box::new(move |value| {
            if let Ok(mut result) = result.lock() {
                *result = Some(value);
            }
        }),
    )
}
type Completed = Rc<dyn Fn(&AppWindow, Result<Option<Vec<u8>>, String>) -> Result<(), String>>;
fn poll(weak: slint::Weak<AppWindow>, pending: Pending, completed: Completed) {
    let Some(window) = weak.upgrade() else { return };
    let result = pending.lock().ok().and_then(|mut result| result.take());
    if let Some(result) = result {
        window.global::<SettingsState>().set_busy(false);
        if let Err(error) = completed(&window, result) {
            fail(&window, error);
        }
    } else {
        slint::Timer::single_shot(Duration::from_millis(50), move || {
            poll(weak, pending, completed)
        });
    }
}
fn fail(window: &AppWindow, error: String) {
    window.global::<SettingsState>().set_busy(false);
    window.global::<SettingsState>().set_error(error.into());
}
