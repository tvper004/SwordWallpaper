#!/bin/bash
set -euo pipefail
cd "$(dirname "$0")/.."
[[ "$(uname -s)" == Darwin ]] || { echo "This package requires macOS." >&2; exit 1; }

cargo build --release
bundle="$PWD/target/Sword Wallpaper.app"
bundle_swordw="$PWD/target/SwordW.app"
helper="$bundle/Contents/Helpers/Aura Wallpaper.app"
mkdir -p "$bundle/Contents/MacOS" "$bundle/Contents/Resources"
mkdir -p "$helper/Contents/MacOS"
install -m 755 target/release/aura "$bundle/Contents/MacOS/"
install -m 755 target/release/aura-bg "$helper/Contents/MacOS/"
ln -sf aura "$bundle/Contents/MacOS/sword"
ln -sf aura "$bundle/Contents/MacOS/Sword"
ln -sf aura "$bundle/Contents/MacOS/swordw"
# Remove the old build layout: a Cocoa helper beside the main executable was
# identified as another foreground Aura launch each time monitors were queried.
rm -f "$bundle/Contents/MacOS/aura-bg"
cp resources/icons/AppIcon.icns "$bundle/Contents/Resources/"
cp LICENSE* THIRD_PARTY_NOTICES.md "$bundle/Contents/Resources/"
python3 - "$bundle/Contents/Info.plist" "$helper/Contents/Info.plist" <<'PY'
import pathlib, plistlib, re, subprocess, sys
version = re.search(r'^version = "([^"]+)"', pathlib.Path('Cargo.toml').read_text(), re.M)[1]
build = subprocess.check_output(['vtool', '-show-build', 'target/release/aura-bg'], text=True)
minimum_macos = re.search(r'minos\s+(\S+)', build)[1]
info = {
    'CFBundleName': 'SwordW',
    'CFBundleDisplayName': 'Sword Wallpaper',
    'CFBundleIdentifier': 'io.github.antwny.swordw',
    'CFBundleExecutable': 'aura',
    'CFBundleIconFile': 'AppIcon',
    'CFBundlePackageType': 'APPL',
    'CFBundleShortVersionString': version,
    'CFBundleVersion': '20261029.1',
    'LSMinimumSystemVersion': minimum_macos,
    'LSApplicationCategoryType': 'public.app-category.utilities',
    'NSHighResolutionCapable': True,
}
pathlib.Path(sys.argv[1]).write_bytes(plistlib.dumps(info))
pathlib.Path(sys.argv[2]).write_bytes(plistlib.dumps({
    'CFBundleName': 'Aura Wallpaper',
    'CFBundleIdentifier': 'io.github.antwny.aura.wallpaper',
    'CFBundleExecutable': 'aura-bg',
    'CFBundlePackageType': 'APPL',
    'CFBundleVersion': info['CFBundleVersion'],
    'LSMinimumSystemVersion': minimum_macos,
    'LSUIElement': True,
}))
PY
codesign --force --sign - "$helper"
codesign --force --sign - "$bundle"
codesign --verify --deep --strict "$bundle"

# Duplicate bundle as SwordW.app as requested
rm -rf "$bundle_swordw"
cp -R "$bundle" "$bundle_swordw"
codesign --force --sign - "$bundle_swordw"

echo "Built: $bundle"
echo "Built: $bundle_swordw"
