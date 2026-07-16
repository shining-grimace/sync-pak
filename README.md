
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

## Running

- (Linux): `cargo run`
- (Android): `./android/gradlew --project-dir android :app:assembleDebug` (or open the `android` directory in Android Studio)
- (Windows 10+): `cargo run`

## Distributing

### Flatpak

Requirements:
- Flatpak
- `org.flatpak.Builder`
- The Freedesktop Rust SDK extension 

See also [Flatpak instructions](flatpak/README.md) for testing processes.

Public release:
- Submit a release manifest with metadata, icon, and stable source to Flathub (not ready yet).

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
