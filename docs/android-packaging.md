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

When the project is opened from the `android` directory in Android Studio, no shell
environment setup is required. The Cargo task receives the SDK selected by Gradle, the JVM
running Gradle as `JAVA_HOME`, and Android platform 36.1 explicitly. This is important
because Android Studio does not necessarily export its bundled JDK to child processes even
though Gradle itself is already using that JDK.

The debug APK is written to `android/app/build/outputs/apk/debug/app-debug.apk`. This package
is for device feasibility testing; it is not the release AAB and does not establish Google
Play readiness.

To build the developer-only capability probes, add `-PfeasibilityProbes=true`. That build
starts the foreground-service probe and opens the system picker shortly after startup. It
logs only capability outcomes and never logs the returned URI. Normal builds do not start
the probes or expose feasibility controls in the application UI.

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

## Foreground execution

Long-running Android transfers use a non-exported foreground service declared with the
`dataSync` type and its type-specific permission. The Rust capability bridge starts it only
from the visible activity. Its low-priority `Sync operations` notification opens SyncPak
when tapped and offers a `Cancel` action; the service is not restarted automatically after
Android stops it.

Android 13+ may keep foreground-service notices out of the notification drawer until the
user grants notification permission, although the service can still start and remains
visible in Android's active-apps interface. The complete UI must request that permission in
context when the user starts their first operation. The service also stops when Android
delivers the API 35+ timeout callback.

The developer-only probe starts the service with the label `Android background probe`
shortly before opening the folder picker. This lets a device test confirm that the service
and notification remain active while the activity is obscured. Use the notification's
`Cancel` action to finish the probe.

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
