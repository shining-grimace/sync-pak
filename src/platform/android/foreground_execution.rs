use std::sync::{Arc, Mutex};

use jni::{
    EnvUnowned, jni_sig, jni_str,
    objects::{JClass, JObject},
};

use crate::{capabilities::CapabilityError, operations::operation_progress::OperationProgress};

static CANCEL_HANDLER: Mutex<Option<Arc<dyn Fn() + Send + Sync>>> = Mutex::new(None);

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

pub fn update(connection_name: &str, progress: &OperationProgress) -> Result<(), CapabilityError> {
    let Some(current_item) = progress.current_item_number() else {
        return Ok(());
    };
    let current_item = i32::try_from(current_item).map_err(|_| CapabilityError::Unexpected)?;
    let total_items =
        i32::try_from(progress.total_items).map_err(|_| CapabilityError::Unexpected)?;
    with_activity(|env, activity| {
        let connection_name = env.new_string(connection_name)?;
        env.call_method(
            activity,
            jni_str!("updateSyncExecution"),
            jni_sig!("(Ljava/lang/String;II)V"),
            &[
                (&connection_name).into(),
                current_item.into(),
                total_items.into(),
            ],
        )?;
        Ok(())
    })
}

pub fn finish_operation(
    connection_name: &str,
    summary: &str,
    operation_id: &str,
) -> Result<(), CapabilityError> {
    with_activity(|env, host| {
        let name = env.new_string(connection_name)?;
        let summary = env.new_string(summary)?;
        let id = env.new_string(operation_id)?;
        env.call_method(
            host,
            jni_str!("finishSyncOperation"),
            jni_sig!("(Ljava/lang/String;Ljava/lang/String;Ljava/lang/String;)V"),
            &[(&name).into(), (&summary).into(), (&id).into()],
        )?;
        Ok(())
    })
}

pub fn finish(connection_name: &str, summary: &str) -> Result<(), CapabilityError> {
    with_activity(|env, activity| {
        let name = env.new_string(connection_name)?;
        let summary = env.new_string(summary)?;
        env.call_method(
            activity,
            jni_str!("finishSyncExecution"),
            jni_sig!("(Ljava/lang/String;Ljava/lang/String;)V"),
            &[(&name).into(), (&summary).into()],
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

/// Installs the active queue's cancellation callback for the notification action.
pub fn set_cancel_handler(handler: Arc<dyn Fn() + Send + Sync>) -> Result<(), CapabilityError> {
    *CANCEL_HANDLER
        .lock()
        .map_err(|_| CapabilityError::Unexpected)? = Some(handler);
    Ok(())
}

pub fn clear_cancel_handler() -> Result<(), CapabilityError> {
    *CANCEL_HANDLER
        .lock()
        .map_err(|_| CapabilityError::Unexpected)? = None;
    Ok(())
}

fn with_activity(
    action: impl FnOnce(&mut jni::Env<'_>, &JObject<'_>) -> jni::errors::Result<()>,
) -> Result<(), CapabilityError> {
    super::context::with_host(action)
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_syncpak_SyncExecutionService_nativeSyncExecutionCancelled<
    'local,
>(
    mut unowned_env: EnvUnowned<'local>,
    _class: JClass<'local>,
) {
    unowned_env
        .with_env(|_| {
            #[cfg(feature = "provider-s3")]
            if let Ok(queue) = super::runtime::queue() {
                if let Some(entry) = queue.activity().iter().find(|entry| {
                    matches!(
                        entry.state,
                        crate::operations::queue::QueueState::Queued
                            | crate::operations::queue::QueueState::Running
                    )
                }) {
                    let _ = queue.cancel(entry.operation_id);
                    return Ok::<(), jni::errors::Error>(());
                }
            }
            let handler = CANCEL_HANDLER
                .lock()
                .ok()
                .and_then(|handler| handler.clone());
            if let Some(handler) = handler {
                handler();
            }
            Ok::<(), jni::errors::Error>(())
        })
        .resolve::<jni::errors::LogErrorAndDefault>();
}
