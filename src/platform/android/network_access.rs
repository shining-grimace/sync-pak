use crate::capabilities::CapabilityError;
use jni::{jni_sig, jni_str};
pub fn has_internet_permission() -> Result<bool, CapabilityError> {
    super::context::with_host(|env, host| {
        env.call_method(
            host,
            jni_str!("hasInternetPermission"),
            jni_sig!("()Z"),
            &[],
        )?
        .z()
    })
}
