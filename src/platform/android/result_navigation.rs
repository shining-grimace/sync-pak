use jni::{jni_sig, jni_str, objects::JString};

pub fn show_requested(window: &crate::AppWindow) {
    let id = super::context::with_host(|env, host| {
        let value = env
            .call_method(
                host,
                jni_str!("peekRequestedResult"),
                jni_sig!("()Ljava/lang/String;"),
                &[],
            )?
            .l()?;
        let value = env.cast_local::<JString>(value)?;
        value.mutf8_chars(env).map(String::from)
    })
    .unwrap_or_default();
    if id.is_empty() {
        return;
    }
    #[cfg(feature = "provider-s3")]
    if super::runtime::queue().is_ok_and(|queue| {
        queue
            .activity()
            .iter()
            .find(|e| e.operation_id.to_string() == id)
            .is_none_or(|e| e.result.is_some())
    }) {
        window.invoke_show_activity_result(id.clone().into());
        let _ = super::context::with_host(|env, host| {
            let id = env.new_string(id)?;
            env.call_method(
                host,
                jni_str!("clearRequestedResult"),
                jni_sig!("(Ljava/lang/String;)V"),
                &[(&id).into()],
            )?;
            Ok(())
        });
    }
}
