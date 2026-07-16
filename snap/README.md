# Snap feasibility package

`snapcraft.yaml` defines an AMD64, strictly confined feasibility package for
Ubuntu 22.04 LTS and later. It intentionally omits the `home` and
`removable-media` plugs: folder selection must happen through the desktop
portal rather than broad filesystem access.

The app requests network and display plugs for cloud transfers and the Slint
desktop window. It also declares `password-manager-service` for the existing
Secret Service credential adapter. That plug is sensitive and normally is not
automatically connected, so a runtime test must include both the connected and
unavailable states.

Build the local prototype with a current Snapcraft installation:

```text
snapcraft --destructive-mode
sudo snap install --dangerous sync-pak_0.1.0_amd64.snap
snap connections sync-pak
snap run sync-pak
```

If Snapcraft builds in an isolated provider, omit `--destructive-mode`. Before
running the credential probe, explicitly inspect the connections. If the
password-manager service is disconnected, test that SyncPak reports protected
storage as unavailable without offering a plaintext fallback.
