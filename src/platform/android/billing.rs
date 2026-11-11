//! Google Play owns transactions; Slint receives only non-secret presentation state.
use std::sync::Mutex;

use jni::{
    EnvUnowned, jni_sig, jni_str,
    objects::{JClass, JString},
};
use slint::ComponentHandle;

use crate::AppWindow;

use super::activity::with_activity;

static WINDOW: Mutex<Option<slint::Weak<AppWindow>>> = Mutex::new(None);

pub fn configure(window: &AppWindow) {
    if let Ok(mut stored) = WINDOW.lock() {
        *stored = Some(window.as_weak());
    }
    window.on_buy_support(|| call_activity("buy"));
    window.on_refresh_support(|| call_activity("refresh"));
    call_activity("snapshot");
}

fn call_activity(action: &str) {
    let result = with_activity(|env, activity| {
        let method = match action {
            "buy" => jni_str!("buySupport"),
            "refresh" => jni_str!("refreshSupport"),
            _ => jni_str!("syncSupportState"),
        };
        env.call_method(activity, method, jni_sig!("()V"), &[])?;
        Ok(())
    });
    if result.is_err() {
        update(
            5,
            String::new(),
            "Google Play billing is unavailable. Please try again.".into(),
        );
    }
}

fn update(phase: i32, price: String, message: String) {
    let weak = WINDOW.lock().ok().and_then(|window| window.clone());
    let _ = slint::invoke_from_event_loop(move || {
        let Some(window) = weak.and_then(|window| window.upgrade()) else {
            return;
        };
        window.set_support_phase(phase);
        window.set_support_price(price.into());
        window.set_support_message(message.into());
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_syncpak_SyncPakActivity_nativeSupportStateChanged<
    'local,
>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
    phase: jni::sys::jint,
    price: JString<'local>,
    message: JString<'local>,
) {
    unowned_env
        .with_env(|env| {
            let price = String::from(price.mutf8_chars(env)?);
            let message = String::from(message.mutf8_chars(env)?);
            update(phase, price, message);
            Ok::<(), jni::errors::Error>(())
        })
        .resolve::<jni::errors::LogErrorAndDefault>();
}
