"""Generate lossless application icons from public/favicon.png (requires Pillow).

Run from any directory: python3 scripts/generate_icons.py
The source is never cropped, stretched, repainted, or overwritten.
"""

from pathlib import Path
import subprocess
import sys

from PIL import Image, ImageOps

ROOT = Path(__file__).resolve().parents[1]
ICONS = ROOT / 'apps/desktop/assets/icons'


def generate():
    source = Image.open(ROOT / 'public/favicon.png')
    source.load()
    profile = source.info.get('icc_profile')
    source = source.convert('RGBA')
    iconset = ICONS / 'Toonvara.iconset'
    iconset.mkdir(parents=True, exist_ok=True)
    for points in (16, 32, 128, 256, 512):
        for scale in (1, 2):
            size = points * scale
            fitted = ImageOps.contain(source, (size, size), Image.Resampling.LANCZOS)
            canvas = Image.new('RGBA', (size, size), (0, 0, 0, 0))
            canvas.paste(fitted, ((size - fitted.width) // 2, (size - fitted.height) // 2))
            suffix = '@2x' if scale == 2 else ''
            canvas.save(iconset / f'icon_{points}x{points}{suffix}.png', icc_profile=profile)
    # PNG sizes are directly useful to desktop launchers on other platforms.
    # Windows ICO includes all common resolutions without changing the source.
    with Image.open(iconset / 'icon_256x256.png') as icon:
        icon.save(ICONS / 'Toonvara.ico', sizes=[(n, n) for n in (16, 24, 32, 48, 64, 128, 256)])
    if sys.platform == 'darwin':
        subprocess.run(['iconutil', '-c', 'icns', str(iconset), '-o', str(ICONS / 'Toonvara.icns')], check=True)
    else:
        with Image.open(iconset / 'icon_512x512@2x.png') as icon:
            icon.save(ICONS / 'Toonvara.icns')
    print(f'Generated icons in {ICONS}')


if __name__ == '__main__':
    generate()
