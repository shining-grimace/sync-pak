#!/usr/bin/env bash
set -euo pipefail

root=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")/.." && pwd)
cd "$root"
if [[ $(uname -s) != Linux || $(uname -m) != x86_64 ]]; then
    echo 'AppImage builds currently require x86-64 Linux.' >&2
    exit 1
fi

# Build on Ubuntu 22.04 for the supported glibc baseline, not on the newest host.
# An explicit target avoids mixing these binaries with Android or host builds.
target=x86_64-unknown-linux-gnu
cargo build --release --locked --target "$target" --bin sync-pak
metadata=$(cargo metadata --no-deps --format-version 1 --locked)
version=$(python3 -c 'import json,sys; print(next(p["version"] for p in json.load(sys.stdin)["packages"] if p["name"] == "sync-pak"))' <<< "$metadata")
target_dir=$(python3 -c 'import json,sys; print(json.load(sys.stdin)["target_directory"])' <<< "$metadata")

mkdir -p "$root/dist" "$target_dir/appimage-tools"
work=$(mktemp -d "$target_dir/appimage.XXXXXX")
trap 'rm -rf -- "$work"' EXIT
appdir="$work/SyncPak.AppDir"
install -m755 "$target_dir/$target/release/sync-pak" "$work/sync-pak"
# Use the build host's binutils; linuxdeploy's older strip cannot read newer ELF
# relocation formats. Distribution libraries are already stripped.
strip --strip-unneeded "$work/sync-pak"
export NO_STRIP=1

# Pin both the release and its checksum; never execute an unverified download.
tool="$target_dir/appimage-tools/linuxdeploy-1-alpha-20251107-1-x86_64.AppImage"
if [[ ! -f $tool ]]; then
    curl --fail --location --retry 3 \
        https://github.com/linuxdeploy/linuxdeploy/releases/download/1-alpha-20251107-1/linuxdeploy-x86_64.AppImage \
        --output "$work/linuxdeploy.AppImage"
    mv "$work/linuxdeploy.AppImage" "$tool"
fi
echo "c20cd71e3a4e3b80c3483cef793cda3f4e990aca14014d23c544ca3ce1270b4d  $tool" | sha256sum --check
chmod +x "$tool"

# Avoid the output plugin's implicit download of a moving runtime release.
runtime="$target_dir/appimage-tools/runtime-20251108-x86_64"
if [[ ! -f $runtime ]]; then
    curl --fail --location --retry 3 \
        https://github.com/AppImage/type2-runtime/releases/download/20251108/runtime-x86_64 \
        --output "$work/runtime"
    mv "$work/runtime" "$runtime"
fi
echo "2fca8b443c92510f1483a883f60061ad09b46b978b2631c807cd873a47ec260d  $runtime" | sha256sum --check

# Winit loads these libraries dynamically, so ELF dependency scanning misses them.
# Graphics drivers, glibc, fonts, certificates and desktop services stay on the host.
libraries=()
for library in libwayland-client.so.0 libwayland-cursor.so.0 libwayland-egl.so.1 \
    libxkbcommon.so.0 libxkbcommon-x11.so.0 libX11.so.6 libX11-xcb.so.1 \
    libXcursor.so.1 libXi.so.6 libXrandr.so.2 libXinerama.so.1 libXrender.so.1 \
    libXfixes.so.3 libfontconfig.so.1; do
    path=$(ldconfig -p | awk -v name="$library" '$1 == name && /x86-64/ && !found { print $NF; found=1 }')
    if [[ -z $path ]]; then
        echo "Missing $library; install the build dependencies in appimage/README.md." >&2
        exit 1
    fi
    libraries+=(--library "$path")
done

export ARCH=x86_64
export APPIMAGE_EXTRACT_AND_RUN=1
export LINUXDEPLOY_OUTPUT_VERSION="$version"
export LDAI_RUNTIME_FILE="$runtime"
export LDAI_OUTPUT="$root/dist/SyncPak-$version-x86_64.AppImage"
"$tool" --appdir "$appdir" \
    --executable "$work/sync-pak" \
    --desktop-file "$root/packaging/linux/com.shininggrimace.SyncPak.desktop" \
    --icon-file "$root/packaging/linux/com.shininggrimace.SyncPak.svg" \
    "${libraries[@]}" --output appimage

cd "$root/dist"
sha256sum "$(basename "$LDAI_OUTPUT")" > "$(basename "$LDAI_OUTPUT").sha256"
echo "Built $LDAI_OUTPUT"
