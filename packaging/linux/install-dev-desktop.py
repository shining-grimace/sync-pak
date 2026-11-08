#!/usr/bin/env python3
"""Install the current checkout's launcher and icon for unpackaged desktop runs."""
import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--data-dir", type=Path, default=Path(
        os.environ.get("XDG_DATA_HOME") or Path.home() / ".local/share"
    ), help="XDG data directory (defaults to the current user's directory)")
    args = parser.parse_args()
    root = Path(__file__).resolve().parents[2]
    metadata = json.loads(subprocess.check_output(
        ["cargo", "metadata", "--offline", "--no-deps", "--format-version", "1"],
        cwd=root,
    ))
    binary = str(Path(metadata["target_directory"]) / "debug/sync-pak")
    # Desktop Exec quoting differs from shell quoting; percent signs are field codes.
    if "\n" in binary or "\r" in binary:
        raise ValueError("Desktop launcher paths cannot contain line breaks")
    quoted = binary.replace("%", "%%")
    for character in ('\\', '"', '`', '$'):
        quoted = quoted.replace(character, '\\' + character)
    quoted = quoted.replace('\\', '\\\\')
    desktop = (root / "packaging/linux/com.shininggrimace.SyncPak.desktop").read_text()
    desktop = desktop.replace("Exec=sync-pak", f'Exec="{quoted}"')
    data_dir = args.data_dir.expanduser().resolve()
    launcher = data_dir / "applications/com.shininggrimace.SyncPak.desktop"
    icon = data_dir / "icons/hicolor/256x256/apps/com.shininggrimace.SyncPak.png"
    launcher.parent.mkdir(parents=True, exist_ok=True)
    icon.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(root / "assets/images/syncpak-logo.png", icon)
    launcher.write_text(desktop)
    print(f"Installed {launcher}")
    print(f"Installed {icon}")
    print("Run cargo run to build and launch this checkout.")


if __name__ == "__main__":
    main()
