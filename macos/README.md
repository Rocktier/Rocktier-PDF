# macOS Platform Assets

This directory contains macOS-specific resources for the PDF Squeeze Tool.

## Contents

- `icons/` - macOS app icons (ICNS format)
- `Info.plist` - macOS app metadata (if needed beyond tauri.conf.json)
- `entitlements.plist` - App Sandbox and permissions (if needed)

## Notes

Tauri will automatically bundle the icons specified in `tauri.conf.json`.
If additional macOS-specific configurations are needed (e.g., App Sandbox,
com.apple.security.files.user-selected.read-write), add them here.
