#!/bin/bash
# ═══════════════════════════════════════════════════════════════
# BizClaw Single-Tenant + Cloudflare Tunnel
# Chạy BizClaw local → tự tạo tunnel → truy cập từ xa
#
# Usage:
#   ./run-local.sh              # Quick tunnel (random URL)
#   ./run-local.sh --named      # Named tunnel (cần cloudflare login)
#   ./run-local.sh --stop       # Dừng tất cả
# ═══════════════════════════════════════════════════════════════

set -euo pipefail

PORT="${BIZCLAW_PORT:-3000}"
BIZCLAW_BIN="./target/release/bizclaw"
PID_DIR="/tmp/bizclaw-local"
TUNNEL_LOG="$PID_DIR/tunnel.log"

# Colors
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

mkdir -p "$PID_DIR"

# ── Stop Mode ──
if [[ "${1:-}" == "--stop" ]]; then
    echo -e "${RED}⏹ Stopping BizClaw + Tunnel...${NC}"
    [[ -f "$PID_DIR/bizclaw.pid" ]] && kill "$(cat $PID_DIR/bizclaw.pid)" 2>/dev/null && echo "  ✅ BizClaw stopped"
    [[ -f "$PID_DIR/tunnel.pid" ]] && kill "$(cat $PID_DIR/tunnel.pid)" 2>/dev/null && echo "  ✅ Tunnel stopped"
    rm -f "$PID_DIR"/*.pid
    exit 0
fi

# ── Prerequisites ──
if ! command -v cloudflared &>/dev/null; then
    echo -e "${RED}❌ cloudflared not found. Install:${NC}"
    echo "   brew install cloudflared"
    exit 1
fi

if [[ ! -f "$BIZCLAW_BIN" ]]; then
    echo -e "${YELLOW}📦 Building BizClaw (release)...${NC}"
    cargo build --release --bin bizclaw
fi

# ── Kill existing ──
[[ -f "$PID_DIR/bizclaw.pid" ]] && kill "$(cat $PID_DIR/bizclaw.pid)" 2>/dev/null || true
[[ -f "$PID_DIR/tunnel.pid" ]] && kill "$(cat $PID_DIR/tunnel.pid)" 2>/dev/null || true

echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo -e "${CYAN}  🦀 BizClaw Single-Tenant + Cloudflare Tunnel${NC}"
echo -e "${CYAN}  📍 Local: http://localhost:${PORT}${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo ""

# ── Step 1: Start BizClaw ──
echo -e "${GREEN}🦀 Starting BizClaw on port ${PORT}...${NC}"
RUST_LOG=info "$BIZCLAW_BIN" serve --port "$PORT" &
BIZCLAW_PID=$!
echo "$BIZCLAW_PID" > "$PID_DIR/bizclaw.pid"

# Wait for BizClaw to be ready
echo -n "  ⏳ Waiting for BizClaw... "
for i in $(seq 1 30); do
    if curl -sf "http://localhost:${PORT}/api/v1/info" &>/dev/null; then
        echo -e "${GREEN}ready!${NC}"
        break
    fi
    if ! kill -0 "$BIZCLAW_PID" 2>/dev/null; then
        echo -e "${RED}FAILED (process died)${NC}"
        exit 1
    fi
    sleep 1
done

# ── Step 2: Start Cloudflare Tunnel ──
if [[ "${1:-}" == "--named" ]]; then
    # Named tunnel — requires `cloudflared login` first
    TUNNEL_NAME="bizclaw-local"
    echo -e "${GREEN}🌐 Creating named tunnel: ${TUNNEL_NAME}...${NC}"
    cloudflared tunnel --no-autoupdate run \
        --url "http://localhost:${PORT}" \
        "$TUNNEL_NAME" \
        > "$TUNNEL_LOG" 2>&1 &
    TUNNEL_PID=$!
else
    # Quick tunnel — no login needed, random URL
    echo -e "${GREEN}🌐 Creating quick tunnel (random URL)...${NC}"
    cloudflared tunnel --no-autoupdate \
        --url "http://localhost:${PORT}" \
        > "$TUNNEL_LOG" 2>&1 &
    TUNNEL_PID=$!
fi
echo "$TUNNEL_PID" > "$PID_DIR/tunnel.pid"

# Wait for tunnel URL
echo -n "  ⏳ Waiting for tunnel URL... "
TUNNEL_URL=""
for i in $(seq 1 15); do
    TUNNEL_URL=$(grep -o 'https://[a-z0-9-]*\.trycloudflare\.com' "$TUNNEL_LOG" 2>/dev/null | head -1 || true)
    if [[ -n "$TUNNEL_URL" ]]; then
        break
    fi
    sleep 1
done

if [[ -z "$TUNNEL_URL" ]]; then
    # Try named tunnel format
    TUNNEL_URL=$(grep -o 'https://[a-z0-9-]*\.[a-z0-9-]*\.cfargotunnel\.com' "$TUNNEL_LOG" 2>/dev/null | head -1 || true)
fi

echo ""
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo -e "${GREEN}  ✅ BizClaw Single-Tenant ONLINE${NC}"
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"
echo ""
echo -e "  🏠 Local:      ${GREEN}http://localhost:${PORT}${NC}"
if [[ -n "$TUNNEL_URL" ]]; then
    echo -e "  🌐 Remote:     ${GREEN}${TUNNEL_URL}${NC}"
    echo ""
    echo -e "  ${YELLOW}📱 Truy cập từ điện thoại/máy khác:${NC}"
    echo -e "     ${GREEN}${TUNNEL_URL}${NC}"
else
    echo -e "  🌐 Remote:     ${YELLOW}(check $TUNNEL_LOG)${NC}"
fi
echo ""
echo -e "  BizClaw PID:   $BIZCLAW_PID"
echo -e "  Tunnel PID:    $TUNNEL_PID"
echo ""
echo -e "  ${YELLOW}Stop:${NC} ./run-local.sh --stop"
echo -e "${CYAN}═══════════════════════════════════════════════════${NC}"

# Save URL for other tools
echo "$TUNNEL_URL" > "$PID_DIR/tunnel.url"

# Keep running — show logs
echo ""
echo -e "${CYAN}📋 Live logs (Ctrl+C to detach, tunnel keeps running):${NC}"
wait "$BIZCLAW_PID"
