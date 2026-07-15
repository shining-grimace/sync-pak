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

#[cfg(feature = "feasibility-probes")]
pub fn schedule_probe() {
    slint::Timer::single_shot(std::time::Duration::from_millis(250), || {
        if let Err(error) = start("Android background probe") {
            eprintln!("Android foreground-service probe could not start: {error}");
        } else {
            eprintln!("Android foreground-service probe started.");
        }
    });
}

pub fn start(connection_name: &str) -> Result<(), CapabilityError> {
    with_activity(|env, activity| {
        let connection_name = env.new_string(connection_name)?;
        env.call_method(
            activity,
            jni_str!("startSyncExecution"),
            jni_sig!("(Ljava/lang/String;)V"),
            &[(&connection_name).into()],
        )?;
        Ok(())
    })
}

pub fn stop() -> Result<(), CapabilityError> {
    with_activity(|env, activity| {
        env.call_method(
            activity,
            jni_str!("stopSyncExecution"),
            jni_sig!("()V"),
            &[],
        )?;
        Ok(())
    })
}

fn with_activity(
    action: impl FnOnce(&mut jni::Env<'_>, &JObject<'_>) -> jni::errors::Result<()>,
) -> Result<(), CapabilityError> {
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
        action(env, &activity)
    })
    .map_err(|_| CapabilityError::Unavailable)
}
