# Snap for Ubuntu

`snapcraft.yaml` defines an AMD64, strictly confined feasibility package for
Ubuntu 22.04 LTS and later. It intentionally omits the `home` and
`removable-media` plugs: folder selection must happen through the desktop
portal rather than broad filesystem access.

The `core24` base supplies the Ubuntu 24.04 userspace even on an Ubuntu 22.04
host. The GNOME extension supplies desktop environment setup and the graphics
content runtime; the package includes the shared desktop launcher and icon.
AppImage is the alternative direct-download format for general Linux systems.

The app requests network and display plugs for cloud transfers and the Slint
desktop window. It also declares `password-manager-service` for the existing
Secret Service credential adapter. That plug is sensitive and normally is not
automatically connected, so a runtime test must include both the connected and
unavailable states.

Build from the repository root with a current Snapcraft installation and its
isolated build provider:

```text
snapcraft
sudo snap install --dangerous sync-pak_0.1.0_amd64.snap
snap connections sync-pak
snap run sync-pak
```

Only use `snapcraft --destructive-mode` in a disposable Ubuntu 24.04 build
environment, matching `core24`. The CI Snap job builds and uploads an unsigned
development artifact; it does not publish to the Snap Store. Before
running the credential probe, explicitly inspect the connections. If the
password-manager service is disconnected, test that SyncPak reports protected
storage as unavailable without offering a plaintext fallback.

To test the connected state:

```sh
sudo snap connect sync-pak:password-manager-service
```

Also test the disconnected state with
`sudo snap disconnect sync-pak:password-manager-service`, then restore the connection. Follow the
[device test plan](../docs/device-test-plan.md) for portal folder persistence,
desktop launch, notifications, upgrade, and credential lifecycle checks.

The package remains `grade: devel` until release validation is complete. Keep
its version aligned with `Cargo.toml`. Public release still requires Snap Store
registration, interface review where required, and publishing through the store.
