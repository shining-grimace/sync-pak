use std::{fs::File, io, sync::Mutex};

use jni::{
    JavaVM,
    objects::{Global, JObject, JString},
    signature::RuntimeMethodSignature,
    strings::JNIString,
};
use slint::android::AndroidApp;

use crate::{
    inventory::{Inventory, InventoryEntry, RelativePath},
    platform::android::document_tree::file::open_file,
    platform::android::document_tree::model::{
        DocumentEntry, DocumentMetadata, status, unavailable,
    },
};

static ANDROID_APP: Mutex<Option<AndroidApp>> = Mutex::new(None);

pub fn initialize(app: AndroidApp) -> io::Result<()> {
    *ANDROID_APP.lock().map_err(|_| unavailable())? = Some(app);
    Ok(())
}

pub fn verify(uri: &str) -> io::Result<()> {
    status(call_one_string(
        "verifyFolder",
        "(Ljava/lang/String;)I",
        uri,
    )?)
}

pub fn inventory(uri: &str) -> io::Result<Inventory> {
    let json = call_json(
        "inventoryFolder",
        "(Ljava/lang/String;)Ljava/lang/String;",
        uri,
        None,
    )?;
    let entries = serde_json::from_str::<Vec<DocumentEntry>>(&json)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid document inventory"))?
        .into_iter()
        .map(InventoryEntry::try_from)
        .collect::<Result<Vec<_>, _>>()?;
    Inventory::new(entries)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid document path"))
}

pub fn metadata(uri: &str, path: &RelativePath) -> io::Result<DocumentMetadata> {
    let json = call_json(
        "folderEntryMetadata",
        "(Ljava/lang/String;Ljava/lang/String;)Ljava/lang/String;",
        uri,
        Some(path.as_str()),
    )?;
    serde_json::from_str(&json)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidData, "invalid document metadata"))
}

pub fn open_read(uri: &str, path: &RelativePath) -> io::Result<File> {
    open_file(call_two_strings(
        "openFolderFileForRead",
        "(Ljava/lang/String;Ljava/lang/String;)I",
        uri,
        path.as_str(),
    )?)
}

pub fn open_write(uri: &str, path: &RelativePath) -> io::Result<File> {
    open_file(call_two_strings(
        "openFolderFileForWrite",
        "(Ljava/lang/String;Ljava/lang/String;)I",
        uri,
        path.as_str(),
    )?)
}

pub fn create_directories(uri: &str, path: &RelativePath) -> io::Result<()> {
    status(call_two_strings(
        "createFolderPath",
        "(Ljava/lang/String;Ljava/lang/String;)I",
        uri,
        path.as_str(),
    )?)
}

pub fn delete(uri: &str, path: &RelativePath) -> io::Result<()> {
    status(call_two_strings(
        "deleteFolderEntry",
        "(Ljava/lang/String;Ljava/lang/String;)I",
        uri,
        path.as_str(),
    )?)
}

fn call_one_string(method: &str, signature: &str, value: &str) -> io::Result<i32> {
    let method = JNIString::from(method);
    let runtime_signature = signature
        .parse::<RuntimeMethodSignature>()
        .map_err(|_| unavailable())?;
    let signature = runtime_signature.method_signature();
    with_activity(|env, activity| {
        let value = env.new_string(value)?;
        env.call_method(activity, &method, &signature, &[(&value).into()])?
            .i()
    })
}

fn call_two_strings(method: &str, signature: &str, first: &str, second: &str) -> io::Result<i32> {
    let method = JNIString::from(method);
    let runtime_signature = signature
        .parse::<RuntimeMethodSignature>()
        .map_err(|_| unavailable())?;
    let signature = runtime_signature.method_signature();
    with_activity(|env, activity| {
        let first = env.new_string(first)?;
        let second = env.new_string(second)?;
        env.call_method(
            activity,
            &method,
            &signature,
            &[(&first).into(), (&second).into()],
        )?
        .i()
    })
}

fn call_json(
    method: &str,
    signature: &str,
    first: &str,
    second: Option<&str>,
) -> io::Result<String> {
    let method = JNIString::from(method);
    let runtime_signature = signature
        .parse::<RuntimeMethodSignature>()
        .map_err(|_| unavailable())?;
    let signature = runtime_signature.method_signature();
    with_activity(|env, activity| {
        let first = env.new_string(first)?;
        let result = if let Some(second) = second {
            let second = env.new_string(second)?;
            env.call_method(
                activity,
                &method,
                &signature,
                &[(&first).into(), (&second).into()],
            )?
        } else {
            env.call_method(activity, &method, &signature, &[(&first).into()])?
        };
        let object = result.l()?;
        if object.is_null() {
            return Err(jni::errors::Error::NullPtr("document provider result"));
        }
        let string = env.cast_local::<JString>(object)?;
        string.mutf8_chars(env).map(String::from)
    })
}

fn with_activity<T>(
    action: impl FnOnce(&mut jni::Env<'_>, &JObject<'_>) -> jni::errors::Result<T>,
) -> io::Result<T> {
    let app = ANDROID_APP
        .lock()
        .map_err(|_| unavailable())?
        .clone()
        .ok_or_else(unavailable)?;
    let vm = JavaVM::singleton().map_err(|_| unavailable())?;
    vm.attach_current_thread(|env| {
        let raw_activity = app.activity_as_ptr() as jni::sys::jobject;
        // SAFETY: AndroidApp guarantees this unowned global reference while `app` is alive.
        let activity = unsafe { env.as_cast_raw::<Global<JObject>>(&raw_activity)? };
        action(env, &activity)
    })
    .map_err(|_| unavailable())
}
