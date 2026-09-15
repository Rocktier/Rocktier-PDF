#!/usr/bin/env bash
# Package Rocktier PDF Editor: build → re-sign with entitlements → DMG.
#
# Two things Tauri's bundler does NOT do for us:
#   1. It never applies `bundle.macOS.entitlements`, and without
#      `com.apple.security.cs.disable-library-validation` the bundled
#      `libpdfium.dylib` is rejected under the hardened runtime.
#   2. The resource layout inside the .app is `Resources/resources/pdfium-runtime`
#      (backend already probes that path).
# So the app must be re-signed by hand after bundling.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")" && pwd)"
APP="$ROOT/src-tauri/target/release/bundle/macos/Rocktier PDF Editor.app"
DMG="$HOME/Downloads/Rocktier PDF Editor.dmg"

echo "▸ Building release .app..."
cd "$ROOT"
npm run tauri:build -- --bundles app

echo "▸ Re-signing with entitlements..."
codesign --force --deep --entitlements "$ROOT/src-tauri/Entitlements.plist" -s - "$APP"
codesign --verify --deep --verbose=1 "$APP"

echo "▸ Building DMG..."
rm -f "$DMG"
if [ -x /opt/homebrew/bin/create-dmg ]; then
  /opt/homebrew/bin/create-dmg \
    --volname "Rocktier PDF Editor" \
    --window-pos 200 120 \
    --window-size 600 400 \
    --icon-size 110 \
    --icon "Rocktier PDF Editor.app" 130 185 \
    --app-drop-link 450 185 \
    --format UDZO \
    --no-internet-enable \
    --skip-jenkins \
    "$DMG" "$APP"
else
  hdiutil create -volname "Rocktier PDF Editor" -srcfolder "$APP" -ov -format UDZO "$DMG" >/dev/null
fi

echo "▸ Delivering to ~/Downloads..."
rm -rf "$HOME/Downloads/Rocktier PDF Editor.app"
cp -R "$APP" "$HOME/Downloads/"

echo ""
echo "▸ DMG ready: $DMG"
echo "▸ Size: $(du -h "$DMG" | cut -f1)"
