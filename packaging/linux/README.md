# Linux desktop icons

AppImage and Snap use `assets/images/syncpak-logo.png`, installed with the icon
name `com.shininggrimace.SyncPak`. The desktop filename and Slint's application
ID use the same name so Wayland desktops can associate running windows with the
launcher. Slint also uses the shared PNG as the running window icon.
Snap uses its prefixed desktop identity (`<snap-instance>_sync-pak`) and an icon
path inside the mounted package so the host desktop can find both.

For local development on COSMIC or another Linux desktop, install the current
checkout's launcher and icon into your user's XDG data directory:

```sh
python3 packaging/linux/install-dev-desktop.py
cargo run
```

The launcher points to this checkout's debug executable, using Cargo metadata to
respect a configured target directory. Run `cargo run` to build it initially.
Reinstall the launcher if you move the checkout or change the target directory.
It uses the same application identity as the packaged app; installing it can
override another user or system launcher with that identity. This is intended
for development rather than alongside an installed release.

Use `--data-dir /some/staging/directory` to stage the desktop files elsewhere.
To remove a default installation, delete
`~/.local/share/applications/com.shininggrimace.SyncPak.desktop` and
`~/.local/share/icons/hicolor/256x256/apps/com.shininggrimace.SyncPak.png`
(or the equivalent files under your configured `XDG_DATA_HOME`).
