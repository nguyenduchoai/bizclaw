#!/bin/bash
# ╔══════════════════════════════════════════════════════════════════╗
# ║  BizClaw — 1-Click Install & Deploy Script                      ║
# ║  Self-hosted AI Agent Platform for Vietnamese Businesses         ║
# ║  Usage: curl -fsSL https://bizclaw.vn/install.sh | bash          ║
# ╚══════════════════════════════════════════════════════════════════╝

set -euo pipefail

# ── Colors ─────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
NC='\033[0m' # No Color
BOLD='\033[1m'

# ── Banner ─────────────────────────────────────────────
echo ""
echo -e "${CYAN}╔══════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}🦞 BizClaw — AI Agent Platform${NC}                  ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  ${GREEN}Self-hosted • Free • Made in Vietnam 🇻🇳${NC}        ${CYAN}║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════╝${NC}"
echo ""

# ── Pre-flight Checks ─────────────────────────────────
info() { echo -e "${BLUE}ℹ${NC}  $1"; }
success() { echo -e "${GREEN}✅${NC} $1"; }
warn() { echo -e "${YELLOW}⚠${NC}  $1"; }
error() { echo -e "${RED}❌${NC} $1"; exit 1; }

check_command() {
    if ! command -v "$1" &> /dev/null; then
        error "$1 is required but not installed. Install it first."
    fi
}

info "Checking system requirements..."
check_command "docker"
check_command "docker" # docker compose is subcommand
check_command "git"
check_command "curl"
success "All requirements met!"

# ── Configuration ──────────────────────────────────────
INSTALL_DIR="${BIZCLAW_DIR:-/opt/bizclaw}"
DOMAIN="${BIZCLAW_DOMAIN:-}"
ADMIN_EMAIL="${BIZCLAW_ADMIN_EMAIL:-admin@bizclaw.local}"
ADMIN_PASSWORD="${BIZCLAW_ADMIN_PASSWORD:-}"

echo ""
info "Installation directory: ${BOLD}${INSTALL_DIR}${NC}"

# Interactive mode if no env vars
if [ -z "$DOMAIN" ]; then
    echo ""
    echo -e "${BOLD}🌐 Domain Configuration${NC}"
    echo "  Enter your domain (e.g., bizclaw.vn)"
    echo "  Leave empty for localhost (development mode)"
    read -p "  Domain: " DOMAIN
    DOMAIN="${DOMAIN:-localhost}"
fi

if [ -z "$ADMIN_PASSWORD" ]; then
    ADMIN_PASSWORD=$(openssl rand -base64 16 | tr -d '/+=' | head -c 16)
    warn "Generated admin password: ${BOLD}${ADMIN_PASSWORD}${NC}"
    echo "  ⚠  Save this password! It won't be shown again."
fi

# ── Clone & Setup ──────────────────────────────────────
echo ""
info "Setting up BizClaw..."

if [ -d "$INSTALL_DIR" ]; then
    warn "Directory ${INSTALL_DIR} exists. Pulling latest..."
    cd "$INSTALL_DIR"
    git pull origin master 2>/dev/null || true
else
    info "Cloning BizClaw..."
    git clone https://github.com/nguyenduchoai/bizclaw-cloud.git "$INSTALL_DIR"
    cd "$INSTALL_DIR"
fi

# ── Generate JWT Secret ───────────────────────────────
JWT_SECRET=$(openssl rand -hex 32)

# ── Create .env File ─────────────────────────────────
cat > "$INSTALL_DIR/.env" <<EOF
# BizClaw Environment Configuration
# Generated at: $(date -u +%Y-%m-%dT%H:%M:%SZ)

# Domain
BIZCLAW_DOMAIN=${DOMAIN}
BIZCLAW_BIND_ALL=1

# Authentication
JWT_SECRET=${JWT_SECRET}
ADMIN_EMAIL=${ADMIN_EMAIL}
ADMIN_PASSWORD=${ADMIN_PASSWORD}

# Platform
BASE_PORT=9001
RUST_LOG=info

# Nginx
NGINX_CONTAINER_NAME=bizclaw-nginx
NGINX_UPSTREAM_HOST=bizclaw

# Optional: PostgreSQL for Enterprise features
# DATABASE_URL=postgres://bizclaw:pass@db:5432/bizclaw_platform
EOF

success "Configuration saved to ${INSTALL_DIR}/.env"

# ── Docker Compose ────────────────────────────────────
echo ""
info "Building and starting BizClaw..."

if [ -f "docker-compose.prod.yml" ]; then
    docker compose -f docker-compose.prod.yml up -d --build
elif [ -f "docker-compose.yml" ]; then
    docker compose up -d --build
else
    error "No docker-compose file found in ${INSTALL_DIR}"
fi

# ── Wait for Health Check ─────────────────────────────
echo ""
info "Waiting for BizClaw to start..."
MAX_WAIT=60
WAITED=0
while [ $WAITED -lt $MAX_WAIT ]; do
    if curl -s "http://localhost:3001/health" | grep -q '"status":"ok"' 2>/dev/null; then
        break
    fi
    sleep 2
    WAITED=$((WAITED + 2))
    echo -n "."
done
echo ""

if [ $WAITED -ge $MAX_WAIT ]; then
    warn "BizClaw is still starting... Check logs: docker compose logs -f"
else
    success "BizClaw is running!"
fi

# ── Print Summary ─────────────────────────────────────
echo ""
echo -e "${CYAN}╔══════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}🎉 BizClaw Installed Successfully!${NC}              ${CYAN}║${NC}"
echo -e "${CYAN}╠══════════════════════════════════════════════════╣${NC}"
if [ "$DOMAIN" = "localhost" ]; then
echo -e "${CYAN}║${NC}  Dashboard:  ${GREEN}http://localhost:3001${NC}               ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  API:        ${GREEN}http://localhost:3001/api${NC}           ${CYAN}║${NC}"
else
echo -e "${CYAN}║${NC}  Dashboard:  ${GREEN}https://${DOMAIN}${NC}                   ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  Platform:   ${GREEN}https://apps.${DOMAIN}${NC}              ${CYAN}║${NC}"
fi
echo -e "${CYAN}║${NC}                                                  ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}Login Credentials:${NC}                              ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  Email:     ${YELLOW}${ADMIN_EMAIL}${NC}                      ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  Password:  ${YELLOW}${ADMIN_PASSWORD}${NC}                   ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}                                                  ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  ${BOLD}Quick Start:${NC}                                    ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  1. Login to Platform                            ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  2. Add an AI Provider (OpenAI, Gemini, etc.)    ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  3. Create your first Agent                      ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  4. Connect Zalo/Telegram channels               ${CYAN}║${NC}"
echo -e "${CYAN}║${NC}  5. Start chatting with your AI!                  ${CYAN}║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "  📖 Docs: ${BLUE}https://github.com/nguyenduchoai/bizclaw-cloud${NC}"
echo -e "  🐛 Issues: ${BLUE}https://github.com/nguyenduchoai/bizclaw-cloud/issues${NC}"
echo -e "  ⭐ Star us: ${BLUE}https://github.com/nguyenduchoai/bizclaw-cloud${NC}"
echo ""
