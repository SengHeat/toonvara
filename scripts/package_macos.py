"""Build a self-contained macOS app: python3 scripts/package_macos.py [--debug]."""

import argparse
import json
import plistlib
from pathlib import Path
import shutil
import subprocess
import sys

from generate_icons import generate, ICONS, ROOT


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--debug', action='store_true', help='Package the development build')
    args = parser.parse_args()
    if sys.platform != 'darwin':
        parser.error('macOS app bundles must be built on macOS')
    generate()
    command = ['cargo', 'build', '--locked', '-p', 'toonvara-desktop', '--message-format=json-render-diagnostics']
    if not args.debug:
        command.append('--release')
    executable = None
    build = subprocess.Popen(command, cwd=ROOT, stdout=subprocess.PIPE, text=True)
    for line in build.stdout:
        message = json.loads(line)
        if message.get('reason') == 'compiler-artifact' and message.get('executable'):
            if message['target']['name'] == 'toonvara-desktop':
                executable = Path(message['executable'])
    if build.wait() != 0 or executable is None:
        raise SystemExit('Desktop build failed; no bundle written')
    metadata = json.loads(subprocess.check_output(
        ['cargo', 'metadata', '--locked', '--no-deps', '--format-version=1'], cwd=ROOT))
    version = next(p['version'] for p in metadata['packages'] if p['name'] == 'toonvara-desktop')
    profile = 'debug' if args.debug else 'release'
    bundle = Path(metadata['target_directory']) / profile / 'bundle/Toonvara.app'
    contents = bundle / 'Contents'
    (contents / 'MacOS').mkdir(parents=True, exist_ok=True)
    (contents / 'Resources').mkdir(parents=True, exist_ok=True)
    shutil.copy2(executable, contents / 'MacOS/toonvara-desktop')
    shutil.copy2(ICONS / 'Toonvara.icns', contents / 'Resources/Toonvara.icns')
    info = {
        'CFBundleDevelopmentRegion': 'en',
        'CFBundleExecutable': 'toonvara-desktop',
        'CFBundleIdentifier': 'com.toonvara.desktop',
        'CFBundleName': 'Toonvara',
        'CFBundleDisplayName': 'Toonvara',
        'CFBundlePackageType': 'APPL',
        'CFBundleIconFile': 'Toonvara.icns',
        'CFBundleShortVersionString': version,
        'CFBundleVersion': version,
        'NSHighResolutionCapable': True,
        'NSPrincipalClass': 'NSApplication',
    }
    with (contents / 'Info.plist').open('wb') as output:
        plistlib.dump(info, output)
    # Ad-hoc signing is sufficient for local use. Distribution signing and
    # notarization require the application's Apple Developer identity.
    subprocess.run(['codesign', '--force', '--sign', '-', str(bundle)], check=True)
    subprocess.run(['codesign', '--verify', '--strict', str(bundle)], check=True)
    print(bundle)


if __name__ == '__main__':
    main()
