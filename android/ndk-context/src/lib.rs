//! Process-owned Android application context for both service and NativeActivity hosts.
//! Both hosts supply global application references that remain alive for the process.
use std::{ffi::c_void, sync::OnceLock};
#[derive(Clone, Copy, Debug)]
pub struct AndroidContext {
    java_vm: *mut c_void,
    context_jobject: *mut c_void,
}
// SAFETY: JavaVM pointers and JNI global references may be shared across threads.
unsafe impl Send for AndroidContext {}
unsafe impl Sync for AndroidContext {}
static CONTEXT: OnceLock<AndroidContext> = OnceLock::new();
impl AndroidContext {
    pub fn vm(self) -> *mut c_void {
        self.java_vm
    }
    pub fn context(self) -> *mut c_void {
        self.context_jobject
    }
}
pub fn android_context() -> AndroidContext {
    *CONTEXT
        .get()
        .expect("Android application context is not initialized")
}
/// # Safety
/// Pointers must identify the process VM and a process-lifetime global Application reference.
/// Multiple hosts may register the same application; the first reference is retained.
pub unsafe fn initialize_android_context(java_vm: *mut c_void, context_jobject: *mut c_void) {
    register(&CONTEXT, java_vm, context_jobject);
}
fn register(slot: &OnceLock<AndroidContext>, java_vm: *mut c_void, context_jobject: *mut c_void) {
    let context = slot.get_or_init(|| AndroidContext {
        java_vm,
        context_jobject,
    });
    assert_eq!(
        context.java_vm, java_vm,
        "Android hosts must use the same VM"
    );
}
/// # Safety
/// Only call after initialization. Context lifetime belongs to the process, not a screen.
pub unsafe fn release_android_context() {}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn repeated_hosts_keep_the_first_application_reference() {
        let slot = OnceLock::new();
        let vm = std::ptr::dangling_mut::<c_void>();
        register(&slot, vm, std::ptr::null_mut());
        register(&slot, vm, std::ptr::dangling_mut::<c_void>());
        assert_eq!(slot.get().unwrap().context(), std::ptr::null_mut());
    }
    #[test]
    fn concurrent_hosts_share_one_application() {
        let slot = std::sync::Arc::new(OnceLock::new());
        let threads: Vec<_> = (0..8)
            .map(|_| {
                let slot = slot.clone();
                std::thread::spawn(move || {
                    register(&slot, std::ptr::null_mut(), std::ptr::null_mut())
                })
            })
            .collect();
        for thread in threads {
            thread.join().unwrap();
        }
        assert!(slot.get().is_some());
    }
}
