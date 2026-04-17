#!/bin/bash
# create-dmg.sh - Create macOS DMG for BizClaw
set -e

APP_NAME="BizClaw"
DMG_NAME="${APP_NAME}.dmg"
VOLUME_NAME="${APP_NAME}"

TEMP_DIR=$(mktemp -d)
mkdir -p "${TEMP_DIR}/${VOLUME_NAME}"

cp -R "target/release/bizclaw.app" "${TEMP_DIR}/${VOLUME_NAME}/"
cp "README.txt" "${TEMP_DIR}/${VOLUME_NAME}/" 2>/dev/null || true

hdiutil create -volname "${VOLUME_NAME}" -srcfolder "${TEMP_DIR}" -ov -format UDZO "${DMG_NAME}"

rm -rf "${TEMP_DIR}"
echo "Created ${DMG_NAME}"
