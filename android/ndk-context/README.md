# Shared Android application context

This local replacement preserves the ndk-context 0.1.1 API used by android-activity
and android-native-keyring-store. SyncPak's Application initializes credentials
before a NativeActivity exists, and a later NativeActivity registers the same VM
and Application again. The upstream initializer panics on that second registration.

Registration here is synchronized with OnceLock, retains the first global
Application reference, and verifies the VM identity. Application context lasts for
the process, including when an Activity closes during a service run. Releasing an
individual screen therefore does not release the process context. Both hosts own
process-lifetime JNI global references. No credential format or encryption changes.
