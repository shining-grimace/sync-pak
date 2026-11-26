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
    if let Ok(queue) = super::runtime::queue() {
        use crate::operations::queue::QueueState;
        let activity = queue.activity();
        let entry = activity
            .iter()
            .find(|entry| entry.operation_id.to_string() == id);
        if entry.is_some_and(|entry| entry.result.is_none()) {
            return;
        }
        let needs_review = entry.is_some_and(|entry| entry.state == QueueState::ReviewRequired);
        if needs_review {
            if activity
                .iter()
                .any(|entry| matches!(entry.state, QueueState::Queued | QueueState::Running))
            {
                return;
            }
            if let Ok(config) = crate::configuration::ConfigStore::for_current_platform()
                && crate::configuration::run_guard::active(config.path()).is_some()
            {
                return;
            }
        }
        window.invoke_show_activity_result(id.clone().into());
        if needs_review {
            window.invoke_retry_activity_result();
        }
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
