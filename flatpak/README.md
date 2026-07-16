# Flatpak feasibility package

`../com.shininggrimace.SyncPak.yml` packages the Linux prototype with the
Freedesktop 25.08 runtime. It intentionally has no broad filesystem permission:
folder selection must use the desktop portal. Network access is required for
provider operations, and the two named session-bus permissions are limited to
Secret Service credential storage and desktop notifications.

This is a local feasibility manifest, not a release submission. A later release
package must add the app metadata, icon, and stable source reference required by
the distribution channel.

The generated `cargo-sources.json` pins every crate in `Cargo.lock` for an
offline Flatpak build. Regenerate it whenever the lockfile changes:

```text
curl -fsSL -o /tmp/flatpak-cargo-generator.py \
  https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
python3 /tmp/flatpak-cargo-generator.py Cargo.lock -o flatpak/cargo-sources.json
```

After installing `org.flatpak.Builder` and the Rust SDK extension from Flathub,
build and run the probe with:

```text
flatpak run --command=flathub-build org.flatpak.Builder --install \
  com.shininggrimace.SyncPak.yml
flatpak run --user com.shininggrimace.SyncPak
```

Use `flatpak info --show-permissions com.shininggrimace.SyncPak` to confirm the
installed sandbox permissions before testing the folder picker, Secret Service
credential probe, and desktop notification probe.
