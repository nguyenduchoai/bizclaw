#!/usr/bin/env bash
# ═══════════════════════════════════════════════════════════
# BizClaw Android APK Build Script
# ═══════════════════════════════════════════════════════════
#
# Usage:
#   ./build-apk.sh           # Debug APK
#   ./build-apk.sh release   # Release APK (needs keystore)
#   ./build-apk.sh clean     # Clean build
#
# Prerequisites:
#   - Android SDK (ANDROID_HOME set)
#   - Java 17+
#   - NDK for llama.cpp native builds
#
# Output:
#   Debug:   app/build/outputs/apk/debug/app-debug.apk
#   Release: app/build/outputs/apk/release/app-release.apk

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
cd "$SCRIPT_DIR"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

log() { echo -e "${BLUE}[BUILD]${NC} $*"; }
ok()  { echo -e "${GREEN}[OK]${NC} $*"; }
warn(){ echo -e "${YELLOW}[WARN]${NC} $*"; }
err() { echo -e "${RED}[ERROR]${NC} $*"; exit 1; }

# ── Check prerequisites ──────────────────────────────────
log "Checking prerequisites..."

if [ -z "${ANDROID_HOME:-}" ]; then
  # Try common locations
  if [ -d "$HOME/Library/Android/sdk" ]; then
    export ANDROID_HOME="$HOME/Library/Android/sdk"
  elif [ -d "$HOME/Android/Sdk" ]; then
    export ANDROID_HOME="$HOME/Android/Sdk"
  else
    err "ANDROID_HOME not set. Install Android SDK first."
  fi
fi
ok "ANDROID_HOME=$ANDROID_HOME"

if ! command -v java &>/dev/null; then
  err "Java not found. Install JDK 17+."
fi
JAVA_VER=$(java -version 2>&1 | head -1 | cut -d'"' -f2 | cut -d'.' -f1)
if [ "$JAVA_VER" -lt 17 ] 2>/dev/null; then
  warn "Java $JAVA_VER detected — JDK 17+ recommended"
fi
ok "Java $(java -version 2>&1 | head -1)"

# ── Build mode ──────────────────────────────────────────
MODE="${1:-debug}"

case "$MODE" in
  clean)
    log "🧹 Cleaning build..."
    ./gradlew clean
    ok "Build cleaned"
    exit 0
    ;;
  release)
    log "🚀 Building RELEASE APK..."
    
    # Check for keystore
    KEYSTORE="${BIZCLAW_KEYSTORE:-$SCRIPT_DIR/release-keystore.jks}"
    if [ ! -f "$KEYSTORE" ]; then
      warn "No keystore found at $KEYSTORE"
      warn "Creating debug-signed release APK..."
      warn "For production, create keystore:"
      warn "  keytool -genkey -v -keystore release-keystore.jks \\"
      warn "    -alias bizclaw -keyalg RSA -keysize 2048 -validity 10000"
      
      # Build unsigned release
      ./gradlew assembleRelease \
        -Pandroid.injected.signing.store.file="" \
        -Pandroid.injected.signing.store.password="" \
        -Pandroid.injected.signing.key.alias="" \
        -Pandroid.injected.signing.key.password=""
    else
      STORE_PASS="${BIZCLAW_STORE_PASSWORD:-}"
      KEY_ALIAS="${BIZCLAW_KEY_ALIAS:-bizclaw}"
      KEY_PASS="${BIZCLAW_KEY_PASSWORD:-}"
      
      if [ -z "$STORE_PASS" ]; then
        echo -n "Keystore password: "
        read -s STORE_PASS
        echo
      fi
      if [ -z "$KEY_PASS" ]; then
        KEY_PASS="$STORE_PASS"
      fi
      
      ./gradlew assembleRelease \
        -Pandroid.injected.signing.store.file="$KEYSTORE" \
        -Pandroid.injected.signing.store.password="$STORE_PASS" \
        -Pandroid.injected.signing.key.alias="$KEY_ALIAS" \
        -Pandroid.injected.signing.key.password="$KEY_PASS"
    fi
    
    APK="app/build/outputs/apk/release/app-release.apk"
    if [ -f "$APK" ]; then
      SIZE=$(du -sh "$APK" | cut -f1)
      ok "📱 Release APK built: $APK ($SIZE)"
    else
      # Try unsigned
      APK="app/build/outputs/apk/release/app-release-unsigned.apk"
      if [ -f "$APK" ]; then
        SIZE=$(du -sh "$APK" | cut -f1)
        ok "📱 Unsigned APK built: $APK ($SIZE)"
        warn "Sign with: apksigner sign --ks release-keystore.jks $APK"
      else
        err "APK not found after build"
      fi
    fi
    ;;
  debug|*)
    log "🔧 Building DEBUG APK..."
    ./gradlew assembleDebug
    
    APK="app/build/outputs/apk/debug/app-debug.apk"
    if [ -f "$APK" ]; then
      SIZE=$(du -sh "$APK" | cut -f1)
      ok "📱 Debug APK built: $APK ($SIZE)"
      
      # Auto-install if device connected
      if command -v adb &>/dev/null; then
        DEVICE_COUNT=$(adb devices | grep -c "device$" || true)
        if [ "$DEVICE_COUNT" -gt 0 ]; then
          log "📲 Installing on connected device..."
          adb install -r "$APK" && ok "Installed on device" || warn "Install failed"
        else
          warn "No Android device connected. Install manually:"
          warn "  adb install $APK"
        fi
      fi
    else
      err "APK not found after build"
    fi
    ;;
esac

echo ""
log "════════════════════════════════════════"
ok "Build complete! 🎉"
log "════════════════════════════════════════"
