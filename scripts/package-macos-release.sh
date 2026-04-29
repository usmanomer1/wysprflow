#!/usr/bin/env bash

set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
APP_PATH="$ROOT_DIR/src-tauri/target/release/bundle/macos/wysprflow.app"
ZIP_PATH="$ROOT_DIR/src-tauri/target/release/bundle/macos/wysprflow.app.zip"
OUT_DIR="$ROOT_DIR/release-assets"
STAGE_DIR="$OUT_DIR/dmg-root"
VERSION="$(node -p "require('$ROOT_DIR/package.json').version")"

case "$(uname -m)" in
  arm64) ARCH_LABEL="aarch64" ;;
  x86_64) ARCH_LABEL="x86_64" ;;
  *) ARCH_LABEL="$(uname -m)" ;;
esac

DMG_PATH="$OUT_DIR/wysprflow_${VERSION}_${ARCH_LABEL}.dmg"
ENTITLEMENTS_PATH="$ROOT_DIR/src-tauri/Entitlements.plist"

if [[ ! -d "$APP_PATH" ]]; then
  echo "App bundle not found at $APP_PATH" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
rm -rf "$STAGE_DIR" "$DMG_PATH"

codesign --force --deep --sign - --entitlements "$ENTITLEMENTS_PATH" "$APP_PATH"
codesign --verify --deep --strict --verbose=2 "$APP_PATH"

ditto -c -k --sequesterRsrc --keepParent "$APP_PATH" "$ZIP_PATH"

mkdir -p "$STAGE_DIR"
ditto "$APP_PATH" "$STAGE_DIR/wysprflow.app"
ln -s /Applications "$STAGE_DIR/Applications"

hdiutil create \
  -volname "wysprflow" \
  -srcfolder "$STAGE_DIR" \
  -ov \
  -format UDZO \
  "$DMG_PATH"

echo "Created:"
echo "  $DMG_PATH"
echo "  $ZIP_PATH"
