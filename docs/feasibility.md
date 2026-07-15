# Cross-platform feasibility

This document records evidence for roadmap milestone 1. A row is complete only after the
prototype has run inside the intended package and security model; compiling alone is not
sufficient.

## Current matrix

The first-release targets are Android 11+ on ARM64, Windows 10+, and supported Linux
distributions capable of running the current Flathub runtime. Linux release testing uses
Ubuntu 22.04 LTS or newer as the Snap baseline, plus the current Ubuntu LTS and Fedora
release.

| Capability | Linux | Android | Windows |
| --- | --- | --- | --- |
| Minimal Slint application | Local build scaffolded | ARM64 debug APK assembled locally with SDK 36.1 and NDK 30; CI configured | Cross-build scaffolded |
| Intended package | Not started (Snap/Flatpak) | Debug APK assembled and its package metadata validated; device run and release AAB pending | Not started (MSIX) |
| File/folder picker | Portal-backed adapter implemented; packaged probe pending | Storage Access Framework bridge implemented; device run pending | Native adapter implemented; packaged probe pending |
| Protected credential storage | Secret Service adapter and test-only probe implemented; packaged run pending | Keystore-backed adapter and test-only probe implemented; packaged run pending | Credential Manager adapter and test-only probe implemented; packaged run pending |
| Background execution | Not applicable | Not started | Not applicable |
| Desktop notification | Adapter and developer-only probe implemented; packaged run pending | Not applicable | Toast adapter and developer-only probe implemented; MSIX run pending |
| Sandbox filesystem access | Not started | Not started | Not started |

Continuous builds compile the shared Slint application on Linux and Windows and package an
ARM64 Android folder-picker probe APK with a minimum SDK of 30, target SDK of 36, and compile
SDK of 36.1. Passing those jobs proves source portability and package assembly, not runtime
behavior.

On 2026-07-15, both the normal and feasibility-probe debug APKs were assembled
locally with Android SDK 36.1 and NDK 30.0.15729638 (beta 2). The resulting APK
was checked for its debug signature, ARM64-only native contents, minimum and
target SDK metadata, and 16 KiB page alignment. Execution and persisted-grant
verification on a physical device remain pending.

## Developer probes

Run the desktop-notification probe with:

```text
cargo run --example desktop_notification --features feasibility-probes
```

The command displays one fixed notification and does not read application data. On Windows,
`SYNCPAK_WINDOWS_APP_ID` can supply the package application user model ID. A run without that
identity is useful only for unpackaged development and does not satisfy the MSIX evidence row.

## Provider evidence

Authenticated list, upload, download, and delete probes are still required for Cloudflare
R2, Backblaze B2, and AWS S3. Each probe must use an isolated test bucket or prefix and
credentials supplied through CI secrets. Logs must contain identifiers only and never
credential values or file contents.

## Design decisions to validate

- Ordinary JSON configuration contains provider metadata and immutable provider and
  connection IDs; the provider ID is the reference for credential JSON held only in
  protected platform storage.
- There is no plaintext credential fallback. An unavailable keyring or keystore is a
  user-visible unavailable state.
- Platform picker results must be rejected when they cannot be represented as UTF-8; paths
  are never converted lossily.
- Package prototypes must verify access through the actual Snap, Flatpak, Android, and MSIX
  sandboxes. An unpackaged desktop test is not equivalent evidence.
- Provider probes should target capability contracts because bucket listing, metadata, and
  multipart support can differ by provider and credential policy.

## Capability findings

- Linux folder selection uses the XDG desktop portal, which is appropriate for both normal
  desktop sessions and Flatpak. Portal availability and persistent access must still be
  tested inside the Flatpak and Snap packages.
- Windows folder selection returns a filesystem path. Its access must be repeated from an
  installed MSIX to expose any package capability differences.
- Android folder selection cannot be modelled as a filesystem path. The Storage Access
  Framework returns a tree content URI and persistable permission grant, so the filesystem
  capability uses a platform-neutral selection type that can carry either a path or URI.
- Android uses a small `NativeActivity` subclass to receive the asynchronous picker result.
  It takes the persistable read/write permissions actually granted, returns cancellation
  separately, and passes only the content tree URI into the shared capability model.
- Protected-storage errors are reduced to redaction-safe categories before reaching the
  UI. The test-only feasibility probe writes a fixed, non-secret JSON value, reads it back,
  and immediately deletes it; developer probes must not appear in the user-facing UI.
- Linux currently targets Secret Service directly. The Flatpak and Snap prototypes must
  confirm that their sandbox policy exposes only the intended credential collection.
- Android's credential adapter uses ciphertext in private preferences backed by a
  non-exportable Android Keystore key. It requires the Android activity context to be
  initialized before the store is opened.
- Windows uses generic credentials in Windows Credential Manager. Persistence and removal
  must be tested under the final MSIX package identity.
- Desktop notifications use an app-owned capability contract and a fixed, non-sensitive
  developer probe; the probe is an example executable and never appears in the user UI.
- Linux notification delivery still needs to be exercised through the desktop session bus
  from the Snap and Flatpak packages.
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
