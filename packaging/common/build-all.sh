#!/bin/bash
# build-all.sh - Master build script for all platforms
set -e

VERSION=$(cat packaging/common/VERSION)
echo "Building BizClaw v${VERSION}..."

cargo build --release

case "$(uname -s)" in
    Darwin*)
        echo "Building macOS DMG..."
        ./packaging/macos/create-dmg.sh
        ;;
    Linux*)
        echo "Building Linux AppImage..."
        ./packaging/linux/build-appimage.sh
        ;;
    MINGW*|MSYS*|CYGWIN*)
        echo "Building Windows installer..."
        ./packaging/windows/build.bat
        ;;
esac

echo "Done! Outputs in target/release/"
