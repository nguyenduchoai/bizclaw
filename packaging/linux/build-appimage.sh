#!/bin/bash
# build-appimage.sh - Build Linux AppImage for BizClaw
set -e

VERSION=$(cat packaging/common/VERSION)
ARCH="x86_64"
APPIMAGE_NAME="BizClaw-${VERSION}-${ARCH}.AppImage"

echo "Building AppImage for BizClaw v${VERSION}..."

cargo build --release --target x86_64-unknown-linux-gnu

mkdir -p build-appimage
cp target/x86_64-unknown-linux-gnu/release/bizclaw build-appimage/
cp packaging/linux/bizclaw.desktop build-appimage/
cp packaging/linux/bizclaw.png build-appimage/ 2>/dev/null || true

cd build-appimage
appimagetool . "${APPIMAGE_NAME}"
cd ..

mv build-appimage/${APPIMAGE_NAME} .
rm -rf build-appimage

echo "Created ${APPIMAGE_NAME}"
