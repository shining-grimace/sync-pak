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
| Minimal Slint application | Local build scaffolded | Cross-build scaffolded | Cross-build scaffolded |
| Intended package | Not started (Snap/Flatpak) | Not started (AAB) | Not started (MSIX) |
| File/folder picker | Portal-backed adapter implemented; packaged probe pending | Android Storage Access Framework adapter required | Native adapter implemented; packaged probe pending |
| Protected credential storage | Secret Service adapter and test-only probe implemented; packaged run pending | Keystore-backed adapter and test-only probe implemented; packaged run pending | Credential Manager adapter and test-only probe implemented; packaged run pending |
| Background execution | Not applicable | Not started | Not applicable |
| Desktop notification | Not started | Not applicable | Not started |
| Sandbox filesystem access | Not started | Not started | Not started |

Continuous builds compile the shared Slint application on Linux and Windows and build its
Android library for AArch64. Passing those jobs proves source portability, not packaging or
runtime behavior.

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

## Open inputs

- The exact colour values for the referenced Stitchy palette need to be recorded in the
  design; an app screenshot is not a stable theme specification.
- Test-account ownership, credential rotation, usage limits, and cleanup policy must be
  established before provider probes are automated.
