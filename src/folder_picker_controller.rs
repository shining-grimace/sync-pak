use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use slint::ComponentHandle;

use crate::{
    AppWindow,
    capabilities::{CapabilityError, FolderPicker, FolderSelection},
    platform::PlatformFolderPicker,
};

type PickResult = Arc<Mutex<Option<Result<Option<FolderSelection>, CapabilityError>>>>;

pub(crate) fn configure(window: &AppWindow) {
    let weak = window.as_weak();
    window.on_select_local_folder(move || select_folder(&weak));
}

fn select_folder(weak: &slint::Weak<AppWindow>) {
    let result: PickResult = Arc::new(Mutex::new(None));
    let completion_result = Arc::clone(&result);
    let completion = Box::new(move |selection| {
        if let Ok(mut pending) = completion_result.lock() {
            *pending = Some(selection);
        }
    });
    if let Err(error) = PlatformFolderPicker.pick_folder(completion) {
        set_error(weak, error);
        return;
    }
    poll_for_selection(weak.clone(), result);
}

fn poll_for_selection(weak: slint::Weak<AppWindow>, result: PickResult) {
    let selection = result.lock().ok().and_then(|mut pending| pending.take());
    match selection {
        Some(Ok(Some(selection))) => match selection.display_value() {
            Ok(path) => {
                if let Some(window) = weak.upgrade() {
                    window.set_connection_form_local(path.into());
                    window.set_status_message(Default::default());
                }
            }
            Err(error) => set_error(&weak, error),
        },
        Some(Ok(None)) => {}
        Some(Err(error)) => set_error(&weak, error),
        None => slint::Timer::single_shot(Duration::from_millis(25), move || {
            poll_for_selection(weak, result)
        }),
    }
}

fn set_error(weak: &slint::Weak<AppWindow>, error: CapabilityError) {
    if let Some(window) = weak.upgrade() {
        window.set_status_message(error.to_string().into());
    }
}
