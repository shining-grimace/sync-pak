use std::sync::Mutex;

use jni::{EnvUnowned, jni_sig, jni_str, objects::JClass};
use slint::ComponentHandle;

use crate::AppWindow;

use super::activity::with_activity;

static WINDOW: Mutex<Option<slint::Weak<AppWindow>>> = Mutex::new(None);

pub fn configure(window: &AppWindow) {
    if let Ok(mut stored) = WINDOW.lock() {
        *stored = Some(window.as_weak());
    }
    update_privacy_choices_available();
    let placement = match (
        window.get_page(),
        window.get_providers_total(),
        window.get_connections_total(),
    ) {
        (1, 0, _) => 1,
        (4, _, 0) => 2,
        _ => 0,
    };
    set_placement(placement);
    if window.get_page() == 0 {
        request_consent();
    }
}

pub fn request_consent() {
    if let Err(error) = with_activity(|env, activity| {
        env.call_method(
            activity,
            jni_str!("requestAdvertisingConsent"),
            jni_sig!("()V"),
            &[],
        )?;
        Ok(())
    }) {
        eprintln!("Android advertising consent request failed: {error}");
    }
}

pub fn set_placement(placement: i32) {
    let _ = with_activity(|env, activity| {
        env.call_method(
            activity,
            jni_str!("setAdvertisingPlacement"),
            jni_sig!("(I)V"),
            &[placement.into()],
        )?;
        Ok(())
    });
}

pub fn set_footer_color(color: i32) {
    let _ = with_activity(|env, activity| {
        env.call_method(
            activity,
            jni_str!("setAdvertisingFooterColor"),
            jni_sig!("(I)V"),
            &[color.into()],
        )?;
        Ok(())
    });
}

pub fn show_privacy_choices() {
    let _ = with_activity(|env, activity| {
        env.call_method(
            activity,
            jni_str!("showAdvertisingPrivacyChoices"),
            jni_sig!("()V"),
            &[],
        )?;
        Ok(())
    });
}

fn update_privacy_choices_available() {
    let available = with_activity(|env, activity| {
        env.call_method(
            activity,
            jni_str!("advertisingPrivacyChoicesAvailable"),
            jni_sig!("()Z"),
            &[],
        )?
        .z()
    })
    .unwrap_or(false);
    update_window(Some(available), None);
}

fn update_window(privacy_choices_available: Option<bool>, banner_inset: Option<i32>) {
    let weak = WINDOW.lock().ok().and_then(|window| window.clone());
    let _ = slint::invoke_from_event_loop(move || {
        let Some(window) = weak.and_then(|window| window.upgrade()) else {
            return;
        };
        if let Some(available) = privacy_choices_available {
            window.set_advertising_privacy_choices_available(available);
        }
        if let Some(inset) = banner_inset {
            window.set_advertising_bottom_inset(inset as f32);
        }
    });
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_syncpak_SyncPakActivity_nativeAdvertisingPrivacyChoicesAvailabilityChanged(
    mut unowned_env: EnvUnowned<'_>,
    _class: JClass<'_>,
    available: jni::sys::jboolean,
) {
    unowned_env
        .with_env(|_| {
            update_window(Some(available), None);
            Ok::<(), jni::errors::Error>(())
        })
        .resolve::<jni::errors::LogErrorAndDefault>();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_syncpak_SyncPakActivity_nativeAdvertisingBannerInsetChanged(
    mut unowned_env: EnvUnowned<'_>,
    _class: JClass<'_>,
    height_dp: jni::sys::jint,
) {
    unowned_env
        .with_env(|_| {
            update_window(None, Some(height_dp));
            Ok::<(), jni::errors::Error>(())
        })
        .resolve::<jni::errors::LogErrorAndDefault>();
}
