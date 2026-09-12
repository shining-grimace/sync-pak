use crate::capabilities::list_file::ListFileCompletion;
use jni::{
    EnvUnowned,
    objects::{JClass, JString},
};
use std::sync::Mutex;

static PENDING: Mutex<Option<ListFileCompletion>> = Mutex::new(None);

pub fn pick(contents: Option<Vec<u8>>, completion: ListFileCompletion) -> Result<(), String> {
    let payload = contents
        .map(String::from_utf8)
        .transpose()
        .map_err(|_| "Export is not UTF-8.".to_owned())?;
    {
        let mut pending = PENDING
            .lock()
            .map_err(|_| "Document picker unavailable.".to_owned())?;
        if pending.is_some() {
            return Err("A document picker is already open.".into());
        }
        *pending = Some(completion);
    }
    let result = super::document_tree::access::call_one_string(
        "pickConnectionList",
        "(Ljava/lang/String;)I",
        payload.as_deref().unwrap_or_default(),
    );
    if !matches!(result, Ok(0)) {
        if let Ok(mut pending) = PENDING.lock() {
            pending.take();
        }
        return Err("The document picker could not be opened.".into());
    }
    Ok(())
}

#[unsafe(no_mangle)]
pub extern "system" fn Java_com_shininggrimace_syncpak_SyncPakActivity_nativeConnectionListResult<
    'local,
>(
    mut env: EnvUnowned<'local>,
    _class: JClass<'local>,
    status: i32,
    contents: JString<'local>,
) {
    let value = env
        .with_env(|env| contents.mutf8_chars(env).map(String::from))
        .resolve::<jni::errors::LogErrorAndDefault>();
    let result = match status {
        0 => Ok(Some(value.into_bytes())),
        1 => Ok(None),
        _ => Err("The JSON document could not be read or saved.".into()),
    };
    if let Some(completion) = PENDING.lock().ok().and_then(|mut pending| pending.take()) {
        completion(result);
    }
}
