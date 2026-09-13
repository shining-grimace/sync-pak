# Cross-platform feasibility

This document records evidence for roadmap milestone 1. A row is complete only after the
prototype has run inside the intended package and security model; compiling alone is not
sufficient.

## Current matrix

The first-release targets are Android 11+ on ARM64, Windows 10+, and supported Linux
x86-64 distributions with glibc 2.35 or newer, Wayland or X11, host graphics drivers,
an XDG desktop portal backend, and a Secret Service keyring. Linux release testing uses
Ubuntu 22.04 LTS or newer as the Snap baseline, plus the current Ubuntu LTS and Fedora
release.

| Capability | Linux | Android | Windows |
| --- | --- | --- | --- |
| Minimal Slint application | AppImage build configured; packaged desktop-session run pending | ARM64 debug APK assembled locally with SDK 36.1 and NDK 30; physical-device run accepted | Cross-build scaffolded |
| Intended package | AppImage script and strict Snap manifest configured in CI; package builds and desktop-session runs pending | Debug APK assembled, validated, and accepted on a physical device; release AAB pending | Not started (MSIX) |
| File/folder picker | Portal-backed adapter implemented; AppImage and Snap runtime probes pending | Storage Access Framework bridge accepted on a physical device | Native adapter implemented; packaged probe pending |
| Protected credential storage | Secret Service adapter and test-only probe implemented; AppImage and Snap runtime probes pending | Keystore-backed adapter and test-only probe implemented; packaged run pending | Credential Manager adapter and test-only probe implemented; packaged run pending |
| Background execution | Not applicable | `dataSync` foreground-service probe packaged; physical-device run pending | Not applicable |
| Desktop notification | Adapter and developer-only probe implemented; AppImage and Snap runtime probes pending | Not applicable | Toast adapter and developer-only probe implemented; MSIX run pending |
| Filesystem access | AppImage uses normal user permissions; strict Snap portal-grant runtime probe pending | SAF document-tree adapter implemented; physical transfer run pending | Not started |

Continuous builds compile the shared Slint application on Linux and Windows, assemble the AppImage and Snap
development packages, and package an ARM64 Android folder-picker probe APK with a minimum SDK
of 30, target SDK of 36, and compile SDK of 36.1. Passing those jobs proves source portability
and package assembly, not runtime behavior.

The former Flatpak prototype built and installed locally on 2026-07-16. That historical
result does not validate the replacement AppImage or Snap packages. Flatpak/Flathub is no
longer a distribution target; its manifest and generated Cargo sources have been removed.
AppImage and Snap runtime evidence remains pending.

During the AppImage migration, the native Linux release build passed locally.
AppImage assembly with linuxdeploy and the pinned runtime, FUSE-free extraction,
desktop metadata validation, and library resolution were checked on Fedora using
the native binary. This is packaging-tool evidence, not an Ubuntu 22.04 baseline
build or an interactive desktop acceptance run. CI adds an X11 startup smoke test;
the full device matrix and Snap installation checks remain pending.

On 2026-07-15, both the normal and feasibility-probe debug APKs were assembled
locally with Android SDK 36.1 and NDK 30.0.15729638 (beta 2). The resulting APK
was checked for its debug signature, ARM64-only native contents, minimum and
target SDK metadata, and 16 KiB page alignment. On 2026-07-15, the physical-device
run and Storage Access Framework behavior were accepted as verified by the tester.

## Developer probes

Run the desktop-notification probe with:

```text
cargo run --example desktop_notification --features feasibility-probes
```

The command displays one fixed notification and does not read application data. On Windows,
`SYNCPAK_WINDOWS_APP_ID` can supply the package application user model ID. A run without that
identity is useful only for unpackaged development and does not satisfy the MSIX evidence row.

## Provider evidence

The `provider_operations` example now supplies the test-only authenticated list, upload,
download, verification, and delete harness for Cloudflare R2, Backblaze B2, and AWS S3.
On 2026-07-17, all three providers passed this probe against isolated test buckets or
prefixes. Each run used the provider's S3-compatible API where applicable and completed
the upload, download, content verification, and cleanup sequence.

Credentialed runs must continue to use isolated test buckets or prefixes and credentials
supplied through CI secrets. Logs must contain no credential values or file contents; see
`docs/provider-probes.md`.

The provider-operation proof is complete. Roadmap milestone 1 remains open until the
pending Linux packaged-runtime and installed Windows MSIX runtime checks in the matrix are
accepted; source builds alone do not meet its packaging/security-model exit criterion.

## Design decisions to validate

- Ordinary JSON configuration contains provider metadata and immutable provider and
  connection IDs; the provider ID is the reference for credential JSON held only in
  protected platform storage.
- There is no plaintext credential fallback. An unavailable keyring or keystore is a
  user-visible unavailable state.
- Platform picker results must be rejected when they cannot be represented as UTF-8; paths
  are never converted lossily.
- Package prototypes must verify the AppImage runtime and the actual Snap, Android, and
  MSIX security models. An unpackaged desktop test is not equivalent evidence.
- Provider probes should target capability contracts because bucket listing, metadata, and
  multipart support can differ by provider and credential policy.

## Capability findings

- Linux folder selection uses the XDG desktop portal in both AppImage and Snap builds.
  Portal availability and persistent access must be tested in each package on Wayland and
  X11, including a missing portal backend and cancelled selection.
- AppImage is not a sandbox: selected paths use normal user filesystem permissions.
  The host provides graphics drivers, fonts, certificates, the session bus, portal backend,
  Secret Service, and notification service. Runtime testing must exercise missing/locked
  services and permission-denied or missing folders without claiming portal revocation
  removes the user's underlying filesystem access.
- The Snap feasibility manifest is strictly confined and deliberately omits `home` and
  `removable-media`. Its Secret Service interface may require a manual user connection, so
  the protected-storage unavailable state is part of the required package test.
- Windows folder selection returns a filesystem path. Its access must be repeated from an
  installed MSIX to expose any package capability differences.
- Android folder selection cannot be modelled as a filesystem path. The Storage Access
  Framework returns a tree content URI and persistable permission grant, so the filesystem
  capability uses a platform-neutral selection type that can carry either a path or URI.
- Android keeps that content URI opaque and routes verification, inventory, reads, writes,
  directory creation, deletion, and archive storage through the document provider. It does not
  require broad storage permission, a `FileProvider`, or a storage-path allowlist XML file.
- Android uses a small `NativeActivity` subclass to receive the asynchronous picker result.
  It takes the persistable read/write permissions actually granted, returns cancellation
  separately, and passes only the content tree URI into the shared capability model.
- Protected-storage errors are reduced to redaction-safe categories before reaching the
  UI. The test-only feasibility probe writes a fixed, non-secret JSON value, reads it back,
  and immediately deletes it; developer probes must not appear in the user-facing UI.
- Linux currently targets Secret Service directly. AppImage must reach the host keyring;
  Snap must exercise the connected and disconnected password-manager-service interface.
  Both must verify credential persistence and the locked/unavailable states.
- Android's credential adapter uses ciphertext in private preferences backed by a
  non-exportable Android Keystore key. It requires the Android activity context to be
  initialized before the store is opened.
- Android sync execution uses a non-exported `dataSync` foreground service. It starts only
  from the visible activity, posts a low-priority cancellable notification, returns
  `START_NOT_STICKY`, and stops itself when Android reports a foreground-service timeout.
- Android 15+ limits all of an app's `dataSync` foreground services to six hours in a
  24-hour period. SyncPak must surface that limit and stop the service immediately when its
  queue becomes idle; it cannot use a boot receiver to bypass the limit.
- Windows uses generic credentials in Windows Credential Manager. Persistence and removal
  must be tested under the final MSIX package identity.
- Desktop notifications use an app-owned capability contract and a fixed, non-sensitive
  developer probe; the probe is an example executable and never appears in the user UI.
- Linux notification delivery still needs to be exercised through the desktop session bus
  from the Snap and AppImage packages.
- Windows notification attribution depends on the application user model ID. The adapter
  accepts the final MSIX identity, while an unpackaged probe may use the notification
  library's development fallback; installed-package behavior remains the required evidence.

## Open inputs

- The exact colour values for the referenced Stitchy palette need to be recorded in the
  design; an app screenshot is not a stable theme specification.
- Test-account ownership, credential rotation, usage limits, and cleanup policy must be
  established before provider probes are automated.
- The Android package still needs release signing ownership and credentials before CI can
  produce the Google Play AAB.
