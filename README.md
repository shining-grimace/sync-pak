
# Building From Source

## Prerequisites

For building the app or running tests:
- Install stable Rust
  - (Windows): Requires the Windows 10 or 11 SDK from the Visual Studio installer
- (Linux): Fontconfig development files:
  - (Fedora): `sudo dnf install fontconfig-devel`
  - (Debian/Ubuntu): `sudo apt install libfontconfig1-dev`)
- (Android): Android Studio, SDK level 36.1, NDK 30, a device or emulator running Android 11+

## Running Tests

`cargo test --all-targets --features feasibility-probes`

For authenticated provider checks, see the isolated [provider probe](docs/provider-probes.md).

### Local provider test credentials

Never store credentials in this repository or SyncPak configuration. Keep one
permission-restricted environment file per provider outside the project:

```text
~/.config/sync-pak/provider-probes/r2.env
~/.config/sync-pak/provider-probes/b2.env
~/.config/sync-pak/provider-probes/s3.env
```

Each file exports `SYNCPAK_PROBE_PROVIDER` (`cloudflare-r2`, `backblaze-b2`, or
`aws-s3`), `SYNCPAK_PROBE_ACCESS_KEY_ID`, `SYNCPAK_PROBE_SECRET_ACCESS_KEY`,
`SYNCPAK_PROBE_BUCKET`, `SYNCPAK_PROBE_PREFIX`, and `SYNCPAK_PROBE_REGION`.
R2 and B2 also require `SYNCPAK_PROBE_ENDPOINT`. Create the directory with mode
`700` and each file with mode `600`, then run:

For Cloudflare R2, set `SYNCPAK_PROBE_REGION=auto`; it is required by the S3
SDK but does not identify an R2 bucket region.

`SYNCPAK_PROBE_PREFIX` is used only by this temporary provider probe, not by
the app's future provider or sync-connection settings. It acts like a remote
folder name inside the test bucket, although object storage stores it as the
start of an object key rather than as a real folder. With
`syncpak-feasibility`, the probe lists that namespace and creates one temporary
object beneath it. Use a simple non-empty value without leading or trailing
slashes; the probe normalizes them if present.

- R2 and AWS S3 treat a prefix as an object-name convention. Use a dedicated
  test bucket where possible; restrict AWS IAM object permissions to that
  prefix as well as the bucket-list request.
- Backblaze B2 application keys can be limited to a file-name prefix. Configure
  the key with the same prefix (or a less restrictive one), because B2 rejects
  list requests outside the key's allowed prefix. Create a normal application
  key in the Backblaze web UI: its key ID and application key work with the
  S3-compatible API. The key name is not used by the probe. Do not use the
  automatically created master application key.

```text
source ~/.config/sync-pak/provider-probes/r2.env
cargo run --example provider_operations --features provider-probes
```

## Running

- (Linux): `cargo run`
- (Android): `./android/gradlew --project-dir android :app:assembleDebug` (or open the `android` directory in Android Studio)
- (Windows 10+): `cargo run`

### Live Preview

To enable live-reloading `.slint` files while the app is running:
`SLINT_LIVE_PREVIEW=1 cargo run --features slint/live-preview`

## Distributing

### AppImage (general Linux)

Build the x86-64 AppImage on Ubuntu 22.04 with `bash appimage/build.sh` after
installing the [AppImage build dependencies](appimage/README.md). The versioned
AppImage and SHA-256 checksum are written to `dist/`. Continuous builds also
provide a `syncpak-appimage-x86_64` artifact.

Download both files into the same directory, then run (substitute the version):

```sh
sha256sum --check SyncPak-0.1.0-x86_64.AppImage.sha256
chmod +x SyncPak-0.1.0-x86_64.AppImage
./SyncPak-0.1.0-x86_64.AppImage
```

Targets maintained x86-64 Linux distributions with glibc 2.35 or newer, Wayland
or X11, OpenGL/EGL drivers, an XDG desktop portal backend, and a Secret Service
keyring. See [AppImage instructions](appimage/README.md) for host requirements,
FUSE alternatives, updates, and release checks. AppImage runs with your normal
user permissions.

Public release: distribute the tested AppImage and checksum through GitHub
Releases or direct downloads. Signing and public release publishing remain part
of the release-integration milestone; CI artifacts are development builds.

### Snap

Requirements:
- Ubuntu 22.04+ with Snapcraft

Follow [Snap instructions](snap/README.md) and inspect interface connections.

Public release:
- Register and publish the signed Snap in the Snap Store (not ready yet).

### Android

Produce a signed AAB and publish through Google Play (not ready yet).

### Windows

Public release:
- Produce a signed MSIX for the Microsoft Store/direct install; the MSIX package is not implemented yet.

## Portable connection lists

Use **Settings → Import JSON** or **Export JSON** to move
connections between devices. Local paths are relative to one saved Local Root (initially your home directory).
Remote destinations match by provider type and bucket name; credentials stay on each device.
See [connection lists](docs/connection-lists.md) for the JSON format and workflows.
