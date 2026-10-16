//! JNI resources owned by Application, valid when no Activity exists.
use crate::capabilities::CapabilityError;
use jni::{
    JavaVM,
    objects::{Global, JObject},
};
use std::{
    path::PathBuf,
    sync::{Mutex, OnceLock},
};
static HOST: OnceLock<Global<JObject<'static>>> = OnceLock::new();
static APPLICATION: OnceLock<Global<JObject<'static>>> = OnceLock::new();
static FILES: OnceLock<PathBuf> = OnceLock::new();
static INITIALIZATION: Mutex<()> = Mutex::new(());

pub fn initialize(
    env: &mut jni::Env<'_>,
    host: JObject<'_>,
    files: PathBuf,
) -> jni::errors::Result<()> {
    let _initialization = INITIALIZATION
        .lock()
        .map_err(|_| jni::errors::Error::NullPtr("platform initialization"))?;
    let vm = env.get_java_vm()?;
    if HOST.get().is_some() {
        return Ok(());
    }
    let context = env
        .call_method(
            &host,
            jni::jni_str!("applicationContext"),
            jni::jni_sig!("()Landroid/content/Context;"),
            &[],
        )?
        .l()?;
    if APPLICATION.get().is_none() {
        let _ = APPLICATION.set(env.new_global_ref(&context)?);
    }
    let application = APPLICATION
        .get()
        .ok_or(jni::errors::Error::NullPtr("application context"))?;
    // SAFETY: APPLICATION owns the JNI global reference for this process's lifetime.
    unsafe {
        ndk_context::initialize_android_context(vm.get_raw().cast(), application.as_raw().cast());
    }
    let _ = FILES.set(files);
    let _ = HOST.set(env.new_global_ref(host)?);
    #[cfg(feature = "provider-s3")]
    super::s3::certificate_verifier::initialize_context(env, context)
        .map_err(|_| jni::errors::Error::NullPtr("TLS initialization"))?;
    Ok(())
}
pub fn files_directory() -> Option<PathBuf> {
    FILES.get().cloned()
}
pub fn with_host<T>(
    action: impl FnOnce(&mut jni::Env<'_>, &JObject<'_>) -> jni::errors::Result<T>,
) -> Result<T, CapabilityError> {
    let host = HOST.get().ok_or(CapabilityError::Unavailable)?;
    JavaVM::singleton()
        .map_err(|_| CapabilityError::Unavailable)?
        .attach_current_thread(|env| action(env, host.as_ref()))
        .map_err(|_| CapabilityError::Unavailable)
}
