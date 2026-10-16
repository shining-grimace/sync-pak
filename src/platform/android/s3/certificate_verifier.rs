#![deny(clippy::expect_used, clippy::panic, clippy::unwrap_used)]

use std::sync::atomic::{AtomicBool, Ordering};

use crate::capabilities::CapabilityError;

static INITIALIZED: AtomicBool = AtomicBool::new(false);

pub fn initialize_context(
    env: &mut jni::Env<'_>,
    context: jni::objects::JObject<'_>,
) -> Result<(), CapabilityError> {
    if !INITIALIZED.load(Ordering::Acquire) {
        rustls_platform_verifier::android::init_with_env(env, context)
            .map_err(|_| CapabilityError::Unavailable)?;
        INITIALIZED.store(true, Ordering::Release);
    }
    Ok(())
}

pub fn is_initialized() -> bool {
    INITIALIZED.load(Ordering::Acquire)
}
