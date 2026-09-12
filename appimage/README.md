# AppImage for Linux

The AppImage is the direct-download Linux package; Snap remains available for
Ubuntu. Current package builds target x86-64 only. Build on Ubuntu 22.04 to keep
the glibc baseline at 2.35. Building on a newer distribution can raise that
baseline even when the resulting file is still named AppImage. Musl-based
distributions and older glibc systems are not supported by this build.

## Build

Install stable Rust with the `x86_64-unknown-linux-gnu` target, then install these
packages in an Ubuntu 22.04 build environment:

```sh
sudo apt-get update
sudo apt-get install --yes build-essential pkg-config libfontconfig1-dev \
  libwayland-client0 libwayland-cursor0 libwayland-egl1 libxkbcommon0 \
  libxkbcommon-x11-0 libx11-6 libx11-xcb1 libxcursor1 libxi6 libxrandr2 \
  libxinerama1 libxrender1 libxfixes3 libegl1 libgl1 \
  curl python3 file desktop-file-utils
rustup target add x86_64-unknown-linux-gnu
bash appimage/build.sh
```

Run from the repository checkout. The script builds the normal application with
`Cargo.lock`, derives its version from Cargo metadata, verifies the pinned
linuxdeploy download, and packages the executable, desktop entry, icon, and
shared libraries. The AppImage runtime is also pinned and checksum-verified.
Dynamically loaded display libraries are explicitly included.
It uses a temporary AppDir so old package contents cannot leak into a new build.
Build tools are cached under Cargo's target directory. Internet access is needed
for uncached crates, linuxdeploy, and the AppImage runtime.
FUSE is not needed to run the build tools.

Output: `dist/SyncPak-<version>-x86_64.AppImage` and its `.sha256` file.
The continuous-build workflow uploads both as `syncpak-appimage-x86_64` and
checks extraction and desktop metadata. Download and extract the artifact ZIP
before following the run instructions; ZIP downloads may lose executable mode.

The shared SVG is a provisional packaging icon, pending final branding.
The build follows the [official native-binary packaging guide](https://docs.appimage.org/packaging-guide/from-source/native-binaries.html)
and uses the [linuxdeploy AppImage output plugin](https://github.com/linuxdeploy/linuxdeploy-plugin-appimage).

## Run and update

Download the AppImage and checksum together, then substitute the version:

```sh
sha256sum --check SyncPak-0.1.0-x86_64.AppImage.sha256
chmod +x SyncPak-0.1.0-x86_64.AppImage
./SyncPak-0.1.0-x86_64.AppImage
```

The desktop needs Wayland or X11, working OpenGL/EGL drivers, fonts/Fontconfig
configuration, trusted CA certificates, a session D-Bus, an XDG desktop portal
with a backend for the desktop, and a Secret Service-compatible keyring (such as
GNOME Keyring). Desktop notifications also require the host notification service.
The AppImage does not bundle these services or graphics drivers. A locked or
missing keyring leaves providers unavailable; credentials have no plaintext fallback.

Normal launch requires FUSE support. If it is unavailable, run without mounting:

```sh
./SyncPak-0.1.0-x86_64.AppImage --appimage-extract-and-run
```

Alternatively, extract with `--appimage-extract` in an empty directory and run
`./squashfs-root/AppRun`. No root installation is needed. AppImage does not
sandbox filesystem access or automatically install a menu shortcut.

To update, close SyncPak, verify the new download, and replace the old AppImage.
There is no automatic updater yet. Configuration stays in the normal XDG config
directory (`~/.config/sync-pak` by default); credentials stay in the host keyring.
Deleting the AppImage leaves both intact. Remove providers in the app first if
you want their stored credentials deleted. AppImage and unpackaged desktop runs
use the same configuration; Snap may use a separate confined configuration path.

## Release validation

Build from the intended release revision on the baseline OS and run the
[device test plan](../docs/device-test-plan.md) on Ubuntu 22.04, current Ubuntu
LTS, and current Fedora, on Wayland and X11. Package assembly alone does not
prove runtime portability. Record results in [feasibility](../docs/feasibility.md).

Publish the tested AppImage and checksum as direct downloads or GitHub Release
assets. CI currently produces development artifacts only; release signing and
publishing are still part of the release-integration milestone. A checksum
detects corruption but is not a publisher signature.
