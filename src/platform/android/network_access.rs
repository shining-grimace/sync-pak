use std::sync::Mutex;

use jni::{
    JavaVM, jni_sig, jni_str,
    objects::{Global, JObject},
};
use slint::android::AndroidApp;

use crate::capabilities::CapabilityError;

static ANDROID_APP: Mutex<Option<AndroidApp>> = Mutex::new(None);

pub fn initialize(app: AndroidApp) -> Result<(), CapabilityError> {
    *ANDROID_APP
        .lock()
        .map_err(|_| CapabilityError::Unexpected)? = Some(app);
    Ok(())
}

pub fn has_internet_permission() -> Result<bool, CapabilityError> {
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
        env.call_method(
            activity,
            jni_str!("hasInternetPermission"),
            jni_sig!("()Z"),
            &[],
        )?
        .z()
    })
    .map_err(|_| CapabilityError::Unavailable)
}
