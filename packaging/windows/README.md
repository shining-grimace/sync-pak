# Windows icons

`build.rs` compiles `syncpak.rc` and embeds `syncpak.ico` into the main executable
for Explorer and shortcuts. Slint separately supplies the running window icon.
MSVC builds use the Windows SDK resource compiler, found in the installed SDK or
on PATH. GNU builds use `<architecture>-w64-mingw32-windres`. Set `RC` to override
the resource compiler executable.

The ICO contains 16, 24, 32, 48, 64, 128, and 256 pixel images. Regenerate it and
the PNG assets after changing `assets/images/syncpak-logo.png`:

```sh
python3 packaging/windows/generate-icons.py
```

The generation script requires Pillow; normal builds use the committed assets
and do not need Python or Pillow.

MSIX packaging is not configured yet. Its eventual `AppxManifest.xml` should
reference `Assets\StoreLogo.png` in `Properties/Logo`, and
`Assets\Square44x44Logo.png` and `Assets\Square150x150Logo.png` in
`uap:VisualElements`' corresponding logo attributes. Copy the provided `Assets`
directory into the package when adding that manifest.
