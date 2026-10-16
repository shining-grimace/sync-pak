use crate::configuration::ConfigStore;
use jni::{
    EnvUnowned,
    objects::{JClass, JObject, JString},
    sys::jstring,
};

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_syncpak_SyncNative_initialize<'l>(
    mut env: EnvUnowned<'l>,
    _: JClass<'l>,
    host: JObject<'l>,
    files: JString<'l>,
) {
    env.with_env(|env| {
        let files = String::from(files.mutf8_chars(env)?);
        super::context::initialize(env, host, files.into())
    })
    .resolve::<jni::errors::LogErrorAndDefault>();
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_syncpak_SyncNative_snapshot<'l>(
    mut env: EnvUnowned<'l>,
    _: JClass<'l>,
) -> jstring {
    env.with_env(|env| {
        let snapshot = snapshot().unwrap_or_else(
            |error| serde_json::json!({"connections": [], "active": null, "error": error}),
        );
        Ok::<_, jni::errors::Error>(env.new_string(snapshot.to_string())?.into_raw())
    })
    .resolve::<jni::errors::LogErrorAndDefault>()
}
fn snapshot() -> Result<serde_json::Value, String> {
    let config = ConfigStore::for_current_platform()
        .map_err(|e| e.to_string())?
        .load()
        .map_err(|e| e.to_string())?;
    let connections = crate::operations::widget_connections::available(&config);
    #[cfg(feature = "provider-s3")]
    let active = super::runtime::queue()
        .map_err(|e| e.to_string())?
        .activity()
        .into_iter()
        .find(|e| {
            matches!(
                e.state,
                crate::operations::queue::QueueState::Queued
                    | crate::operations::queue::QueueState::Running
            )
        })
        .map(
            |e| serde_json::json!({"id": e.plan.connection_id, "name": e.snapshot.connection_name}),
        );
    #[cfg(not(feature = "provider-s3"))]
    let active: Option<serde_json::Value> = None;
    Ok(serde_json::json!({
        "connections": connections,
        "active": active,
        "appearance": config.appearance,
    }))
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_syncpak_SyncNative_start<'l>(
    mut env: EnvUnowned<'l>,
    _: JClass<'l>,
    id: JString<'l>,
) -> jstring {
    env.with_env(|env| {
        let id = String::from(id.mutf8_chars(env)?);
        #[cfg(feature = "provider-s3")]
        let result = super::runtime::start(&id).map_err(|e| e.to_string());
        #[cfg(not(feature = "provider-s3"))]
        let result: Result<(), String> = Err(format!("Provider support is unavailable for {id}."));
        Ok::<_, jni::errors::Error>(env.new_string(result.err().unwrap_or_default())?.into_raw())
    })
    .resolve::<jni::errors::LogErrorAndDefault>()
}
