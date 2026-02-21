#!/bin/bash

# ==============================================================================
# 🚀 BIZINO AI DEV - Antigravity / Gemini Kit
# ==============================================================================
# Installation Script for Antigravity / Google Gemini
# Version: 3.0.0
# ==============================================================================

set -e

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
WHITE='\033[1;37m'
MAGENTA='\033[0;35m'
NC='\033[0m'

# Script directory
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Global workflows directory
GLOBAL_WORKFLOWS_DIR="$HOME/.gemini/antigravity/global_workflows"

clear
echo -e "${CYAN}"
echo "╔═══════════════════════════════════════════════════════════════════╗"
echo "║   ⚡ BIZINO AI DEV v3.0 - Antigravity / Gemini Kit                ║"
echo "║   Premium Software Company Agent System                           ║"
echo "╚═══════════════════════════════════════════════════════════════════╝"
echo -e "${NC}"
echo ""

# ==============================================================================
# STEP 1: Always Install/Update Global Workflows
# ==============================================================================
echo -e "${WHITE}🌐 Updating Global Workflows...${NC}"
echo -e "${CYAN}   → ~/.gemini/antigravity/global_workflows/${NC}"
echo ""

# Always install global
INSTALL_GLOBAL=true

# ==============================================================================
# STEP 2: Ask about Project Installation
# ==============================================================================
echo -e "${WHITE}📁 Bạn có muốn cài vào một project cụ thể không?${NC}"
echo ""
echo -e "  ${GREEN}Y)${NC} Có - Chọn thư mục để cài đặt"
echo -e "  ${GREEN}N)${NC} Không - Chỉ cài Global thôi"
echo ""
read -p "$(echo -e ${YELLOW}Cài vào project? [Y/n]: ${NC})" INSTALL_PROJECT_CHOICE

case $INSTALL_PROJECT_CHOICE in
    [Nn]*)
        INSTALL_PROJECT=false
        ;;
    *)
        INSTALL_PROJECT=true
        ;;
esac

# ==============================================================================
# STEP 2: Get Project Directory (if needed)
# ==============================================================================
if [ "$INSTALL_PROJECT" = true ]; then
    echo ""
    echo -e "${WHITE}📁 Nhập đường dẫn thư mục project:${NC}"
    echo -e "${CYAN}   (Để trống = thư mục cha của kit: $(dirname "$SCRIPT_DIR"))${NC}"
    echo ""
    read -p "$(echo -e ${YELLOW}Path: ${NC})" USER_TARGET_DIR
    
    if [ -z "$USER_TARGET_DIR" ]; then
        TARGET_DIR="$(dirname "$SCRIPT_DIR")"
    else
        # Expand ~ to home directory
        TARGET_DIR="${USER_TARGET_DIR/#\~/$HOME}"
    fi
    
    # Convert to absolute path
    if [[ "$TARGET_DIR" != /* ]]; then
        TARGET_DIR="$(pwd)/$TARGET_DIR"
    fi
    
    # Create target if not exists
    if [ ! -d "$TARGET_DIR" ]; then
        echo -e "${YELLOW}Tạo thư mục: $TARGET_DIR${NC}"
        mkdir -p "$TARGET_DIR"
    fi
    
    echo -e "${GREEN}✓ Target: $TARGET_DIR${NC}"
fi

echo ""
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${WHITE}🚀 BẮT ĐẦU CÀI ĐẶT...${NC}"
echo -e "${BLUE}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""

# ==============================================================================
# Install Global Workflows
# ==============================================================================
if [ "$INSTALL_GLOBAL" = true ]; then
    echo -e "${MAGENTA}[1/2] Installing Global Workflows...${NC}"
    
    # Create global workflows directory
    mkdir -p "$GLOBAL_WORKFLOWS_DIR"
    
    # Copy workflows
    if [ -d "$SCRIPT_DIR/.agent/workflows" ]; then
        cp "$SCRIPT_DIR/.agent/workflows/"*.md "$GLOBAL_WORKFLOWS_DIR/" 2>/dev/null || true
        WORKFLOW_COUNT=$(ls "$GLOBAL_WORKFLOWS_DIR/"*.md 2>/dev/null | wc -l | tr -d ' ')
        echo -e "${GREEN}  ✓ $WORKFLOW_COUNT workflows installed globally${NC}"
    fi
    
    echo -e "${GREEN}  ✓ Location: $GLOBAL_WORKFLOWS_DIR${NC}"
    echo ""
fi

# ==============================================================================
# Install to Project
# ==============================================================================
if [ "$INSTALL_PROJECT" = true ]; then
    echo -e "${MAGENTA}[2/2] Installing to Project: $TARGET_DIR${NC}"
    
    # Copy .agent directory (contains workflows, roles, agents, skills)
    if [ -d "$SCRIPT_DIR/.agent" ]; then
        cp -r "$SCRIPT_DIR/.agent" "$TARGET_DIR/"
        echo -e "${GREEN}  ✓ .agent/ directory copied${NC}"
        echo -e "${CYAN}      → workflows/ (22 files)${NC}"
        echo -e "${CYAN}      → agents/ (17 files)${NC}"
        echo -e "${CYAN}      → roles/ (7 files)${NC}"
        echo -e "${CYAN}      → skills/ (44+ skills)${NC}"
    fi
    
    # Copy GEMINI.md to root
    if [ -f "$SCRIPT_DIR/GEMINI.md" ]; then
        cp "$SCRIPT_DIR/GEMINI.md" "$TARGET_DIR/"
        echo -e "${GREEN}  ✓ GEMINI.md copied${NC}"
    fi
    
    # Copy README and GETTING_STARTED to .agent
    [ -f "$SCRIPT_DIR/README.md" ] && cp "$SCRIPT_DIR/README.md" "$TARGET_DIR/.agent/"
    [ -f "$SCRIPT_DIR/GETTING_STARTED.md" ] && cp "$SCRIPT_DIR/GETTING_STARTED.md" "$TARGET_DIR/.agent/"
    
    # Copy docs directory
    if [ -d "$SCRIPT_DIR/docs" ]; then
        cp -r "$SCRIPT_DIR/docs" "$TARGET_DIR/"
        echo -e "${GREEN}  ✓ docs/ copied${NC}"
    fi
    
    # Copy guide directory
    if [ -d "$SCRIPT_DIR/guide" ]; then
        cp -r "$SCRIPT_DIR/guide" "$TARGET_DIR/"
        echo -e "${GREEN}  ✓ guide/ copied${NC}"
    fi
    
    # Create common directories
    mkdir -p "$TARGET_DIR/plans/specs"
    mkdir -p "$TARGET_DIR/plans/active"
    mkdir -p "$TARGET_DIR/plans/reports"
    mkdir -p "$TARGET_DIR/plans/archive"
    mkdir -p "$TARGET_DIR/docs/architecture"
    mkdir -p "$TARGET_DIR/docs/api"
    mkdir -p "$TARGET_DIR/docs/database"
    mkdir -p "$TARGET_DIR/docs/reports"
    
    # Copy templates
    if [ -d "$SCRIPT_DIR/templates/plans" ]; then
        cp -r "$SCRIPT_DIR/templates/plans/"* "$TARGET_DIR/plans/" 2>/dev/null || true
        echo -e "${GREEN}  ✓ Plan templates copied${NC}"
    fi
    
    if [ -d "$SCRIPT_DIR/templates/docs" ]; then
        cp -r "$SCRIPT_DIR/templates/docs/"* "$TARGET_DIR/docs/" 2>/dev/null || true
        echo -e "${GREEN}  ✓ Doc templates copied${NC}"
    fi
    
    touch "$TARGET_DIR/plans/specs/.gitkeep"
    touch "$TARGET_DIR/plans/active/.gitkeep"
    touch "$TARGET_DIR/plans/reports/.gitkeep"
    touch "$TARGET_DIR/plans/archive/.gitkeep"
    
    echo -e "${GREEN}  ✓ Project structure created${NC}"
    echo ""
fi

# ==============================================================================
# Summary
# ==============================================================================
echo ""
echo -e "${GREEN}╔═══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║      ✅ BIZINO AI DEV v3.0 INSTALLED SUCCESSFULLY!                ║${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════════════════════════════════╝${NC}"
echo ""

if [ "$INSTALL_GLOBAL" = true ]; then
    echo -e "${WHITE}🌐 Global Workflows:${NC}"
    echo -e "   ${CYAN}$GLOBAL_WORKFLOWS_DIR${NC}"
    echo ""
fi

if [ "$INSTALL_PROJECT" = true ]; then
    echo -e "${WHITE}📁 Project Installation:${NC}"
    echo -e "   ${CYAN}$TARGET_DIR${NC}"
    echo ""
fi

echo -e "${WHITE}⚡ Quick Start Commands:${NC}"
echo ""
echo -e "  ${GREEN}/cook${NC}       🔥 Full Auto Pipeline (Idea → MVP)"
echo -e "  ${GREEN}/recap${NC}      🧠 Restore context (Bắt đầu ngày mới)"
echo -e "  ${GREEN}/save-brain${NC} 💾 Persist context (Kết thúc ngày)"
echo -e "  ${GREEN}/audit${NC}      🏥 Health check"
echo -e "  ${GREEN}/run${NC}        ▶️ Start application"
echo -e "  ${GREEN}/deploy${NC}     🚀 Deploy to production"
echo ""
echo -e "${WHITE}📖 Documentation:${NC}"
echo -e "   ${CYAN}GEMINI.md${NC} - System configuration"
echo -e "   ${CYAN}.agent/workflows/${NC} - All available workflows"
echo -e "   ${CYAN}.agent/agents/${NC} - Specialized agents"
echo ""
echo -e "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${WHITE}  Bizino AI DEV v3.0 - Premium Software Development, Automated${NC}"
echo -e "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
