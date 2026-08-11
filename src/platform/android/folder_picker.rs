use std::sync::Mutex;

use jni::{
    EnvUnowned, JavaVM, jni_sig, jni_str,
    objects::{Global, JClass, JObject, JString},
};
use slint::android::AndroidApp;

use crate::capabilities::{CapabilityError, FolderPickerCompletion, FolderSelection};

static ANDROID_APP: Mutex<Option<AndroidApp>> = Mutex::new(None);
static PENDING_PICK: Mutex<Option<FolderPickerCompletion>> = Mutex::new(None);

pub fn initialize(app: AndroidApp) -> Result<(), CapabilityError> {
    let vm = app.vm_as_ptr();
    if vm.is_null() {
        return Err(CapabilityError::Unavailable);
    }
    // SAFETY: AndroidApp owns this process's VM, whose pointer remains valid for the process.
    let _ = unsafe { JavaVM::from_raw(vm.cast()) };
    *ANDROID_APP
        .lock()
        .map_err(|_| CapabilityError::Unexpected)? = Some(app);
    *PENDING_PICK
        .lock()
        .map_err(|_| CapabilityError::Unexpected)? = None;
    Ok(())
}

#[cfg(feature = "feasibility-probes")]
pub fn schedule_probe() {
    slint::Timer::single_shot(std::time::Duration::from_millis(500), || {
        let completion = Box::new(|result| match result {
            Ok(Some(_)) => eprintln!("Android folder-picker probe selected a tree URI."),
            Ok(None) => eprintln!("Android folder-picker probe was cancelled."),
            Err(error) => eprintln!("Android folder-picker probe failed: {error}"),
        });
        if let Err(error) = pick_folder(completion) {
            eprintln!("Android folder-picker probe could not start: {error}");
        }
    });
}

pub fn pick_folder(completion: FolderPickerCompletion) -> Result<(), CapabilityError> {
    {
        let mut pending = PENDING_PICK
            .lock()
            .map_err(|_| CapabilityError::Unexpected)?;
        if pending.is_some() {
            return Err(CapabilityError::Busy);
        }
        *pending = Some(completion);
    }

    if let Err(error) = launch_picker() {
        let _ = PENDING_PICK.lock().map(|mut pending| pending.take());
        return Err(error);
    }
    Ok(())
}

fn launch_picker() -> Result<(), CapabilityError> {
    let app = ANDROID_APP
        .lock()
        .map_err(|_| CapabilityError::Unexpected)?
        .clone()
        .ok_or(CapabilityError::Unavailable)?;
    let vm = JavaVM::singleton().map_err(|_| CapabilityError::Unavailable)?;

    vm.attach_current_thread(|env| {
        let raw_activity = app.activity_as_ptr() as jni::sys::jobject;
        // SAFETY: AndroidApp guarantees this unowned global reference while `app` is alive.
        let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&raw_activity)? };
        env.call_method(activity, jni_str!("pickFolder"), jni_sig!("()V"), &[])?;
        Ok::<(), jni::errors::Error>(())
    })
    .map_err(|_| CapabilityError::Unavailable)
}

fn complete(result: Result<Option<FolderSelection>, CapabilityError>) {
    let completion = PENDING_PICK
        .lock()
        .ok()
        .and_then(|mut pending| pending.take());
    if let Some(completion) = completion {
        let _ = slint::invoke_from_event_loop(move || completion(result));
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_syncpak_SyncPakActivity_nativeFolderPicked<
    'local,
>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
    uri: JString<'local>,
) {
    let uri = unowned_env
        .with_env(|env| uri.mutf8_chars(env).map(String::from))
        .resolve::<jni::errors::LogErrorAndDefault>();
    if uri.is_empty() {
        complete(Err(CapabilityError::Unexpected));
    } else {
        complete(Ok(Some(FolderSelection::AndroidTreeUri(uri))));
    }
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_syncpak_SyncPakActivity_nativeFolderPickCancelled<
    'local,
>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
) {
    unowned_env
        .with_env(|_| {
            complete(Ok(None));
            Ok::<(), jni::errors::Error>(())
        })
        .resolve::<jni::errors::LogErrorAndDefault>();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_syncpak_SyncPakActivity_nativeFolderPickFailed<
    'local,
>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
) {
    unowned_env
        .with_env(|_| {
            complete(Err(CapabilityError::Unavailable));
            Ok::<(), jni::errors::Error>(())
        })
        .resolve::<jni::errors::LogErrorAndDefault>();
}
