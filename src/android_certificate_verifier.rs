#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::sync::atomic::{AtomicBool, Ordering};

use jni::{
    JavaVM, jni_sig, jni_str,
    objects::{Global, JObject},
};
use slint::android::AndroidApp;

use crate::capabilities::CapabilityError;

static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn initialize(app: &AndroidApp) -> Result<(), CapabilityError> {
    let vm = JavaVM::singleton().map_err(|_| CapabilityError::Unavailable)?;
    vm.attach_current_thread(|env| {
        let raw_activity = app.activity_as_ptr() as jni::sys::jobject;
        // SAFETY: AndroidApp owns this global reference for at least this initialization call.
        let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&raw_activity)? };
        let context = env
            .call_method(
                activity,
                jni_str!("getApplicationContext"),
                jni_sig!(() -> android.content.Context),
                &[],
            )?
            .l()?;
        rustls_platform_verifier::android::init_with_env(env, context)?;
        Ok::<(), jni::errors::Error>(())
    })
    .map_err(|_| CapabilityError::Unavailable)?;
    INITIALIZED.store(true, Ordering::Release);
    Ok(())
}

pub fn is_initialized() -> bool {
    INITIALIZED.load(Ordering::Acquire)
}
