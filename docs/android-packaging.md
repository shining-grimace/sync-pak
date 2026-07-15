# Android feasibility package

SyncPak's Android feasibility build targets Android 11+ (`minSdkVersion` 30), targets API 35,
and includes only the ARM64 ABI. The package identity is `com.shininggrimace.syncpak`.

## Local debug build

Install Rust's AArch64 Android target, Android SDK platform 35, Android build tools 35.0.0,
an Android NDK, and `cargo-apk` 0.10.0. With `ANDROID_HOME` and `ANDROID_NDK_ROOT` set, run:

```text
cargo apk build --target aarch64-linux-android --lib
```

The debug APK is written below `target/debug/apk/`. This package is for device feasibility
testing; it is not the release AAB and does not establish Google Play readiness.

## Folder-selection prerequisite

The current Slint backend hosts the app in Android `NativeActivity`. A Storage Access
Framework tree selection returns through an Android activity-result callback, which is not
part of the Rust event stream exposed by that host. The next Android slice must add a small
activity bridge that:

1. launches `ACTION_OPEN_DOCUMENT_TREE`;
2. receives the result URI and read/write grant flags;
3. calls `takePersistableUriPermission`;
4. returns the URI to Rust as `FolderSelection::AndroidTreeUri`.

Until all four steps are present, Android folder selection remains unsupported rather than
pretending a content URI is a filesystem path.
