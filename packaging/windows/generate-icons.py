#!/usr/bin/env python3
"""Regenerate Windows icon assets from the shared app icon (requires Pillow)."""
from pathlib import Path
from PIL import Image

directory = Path(__file__).resolve().parent
source = directory.parents[1] / "assets/images/syncpak-logo.png"
with Image.open(source) as image:
    image = image.convert("RGBA")
    image.save(directory / "syncpak.ico", sizes=[
        (16, 16), (24, 24), (32, 32), (48, 48), (64, 64), (128, 128), (256, 256),
    ])
    assets = directory / "Assets"
    assets.mkdir(exist_ok=True)
    for name, size in [("Square44x44Logo", 44), ("Square150x150Logo", 150), ("StoreLogo", 50)]:
        image.resize((size, size), Image.Resampling.LANCZOS).save(assets / f"{name}.png")
