# Android feasibility package

SyncPak's Android feasibility build targets Android 11+ (`minSdkVersion` 30), targets API 36,
compiles against Android 16 SDK 36.1, and includes only the ARM64 ABI. The package identity is
`com.shininggrimace.syncpak`.

## Local debug build

Install Rust's AArch64 Android target, Android SDK platform 36.1, Android build tools 36.0.0,
a side-by-side Android NDK, and JDK 17 or newer. The build uses `ANDROID_NDK_HOME` or
`ANDROID_NDK_ROOT` when set; otherwise it selects the highest installed NDK below the SDK.
Set `ANDROID_HOME` if the SDK is not in `$HOME/Android/Sdk`, then run:

```text
JAVA_HOME=/path/to/android-studio/jbr ./android/gradlew --project-dir android :app:assembleDebug
```

The debug APK is written to `android/app/build/outputs/apk/debug/app-debug.apk`. This package
is for device feasibility testing; it is not the release AAB and does not establish Google
Play readiness.

To build the developer-only folder-picker probe, add `-PfeasibilityProbes=true`. That build
opens the system picker shortly after startup and logs only selected, cancelled, or failed;
it never logs the returned URI. Normal builds do not start the probe and expose no feasibility
controls in the application UI.

## Folder selection

The Slint backend is hosted by `SyncPakActivity`, a small `NativeActivity` subclass. Its
Storage Access Framework bridge:

1. launches `ACTION_OPEN_DOCUMENT_TREE`;
2. receives the result URI and read/write grant flags;
3. calls `takePersistableUriPermission`;
4. returns the URI to Rust as `FolderSelection::AndroidTreeUri`.

Folder selection is asynchronous because Android delivers the result after the system picker
closes. Only one request may be active, and cancellation returns no selection. The bridge
does not request broad storage permissions.

## Physical-device probe

An emulator is not required. An ARM64 device running Android 11 or newer is the
preferred final check for the current target. Enable USB debugging, connect the
device, and accept its authorization prompt, then confirm that ADB can see it:

```text
$HOME/Android/Sdk/platform-tools/adb devices -l
$HOME/Android/Sdk/platform-tools/adb shell getprop ro.product.cpu.abi
$HOME/Android/Sdk/platform-tools/adb shell getprop ro.build.version.sdk
```

Install the feasibility-probe APK and launch SyncPak. It opens Android's folder
picker shortly after startup. Selecting a folder should return to the app and
persist the URI permission; logs report only the outcome and never the URI:

```text
$HOME/Android/Sdk/platform-tools/adb install -r android/app/build/outputs/apk/debug/app-debug.apk
$HOME/Android/Sdk/platform-tools/adb logcat | rg 'Android folder-picker probe'
```
