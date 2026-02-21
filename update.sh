#!/bin/bash

# ==============================================================================
# 🔄 BIZINO AI DEV v3.1 - Universal Auto-Update
# ==============================================================================
# Automatically pull latest changes and update your selected platform
# Version: 3.1.0
# Updated: 2026-01-18
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

# Configuration
REPO_URL="https://github.com/bizino/Bizino-Dev_Kit.git"
BIZINO_HOME="$HOME/.bizino-dev-kit"
CACHE_DIR="$BIZINO_HOME/cache"
CONFIG_FILE="$BIZINO_HOME/config"

# Global directories for each platform
GEMINI_GLOBAL_WORKFLOWS="$HOME/.gemini/antigravity/global_workflows"
CLAUDE_GLOBAL_COMMANDS="$HOME/.claude/commands"
CLAUDE_GLOBAL_CONFIG="$HOME/.claude"
VSCODE_GLOBAL_DIR="$HOME/.vscode-copilot/workflows"

# Embedded token (base64 encoded)
_ENCODED_TOKEN="Z2hwX0s2emx5UWRaYVo2aGdFT3VGcGxMaW9ZVGdLeDhCUDNXQ1Nhawo="
_EMBEDDED_TOKEN=$(echo "$_ENCODED_TOKEN" | base64 -d 2>/dev/null | tr -d '\n' || echo "")

# ==============================================================================
# Functions
# ==============================================================================

show_header() {
    clear
    echo -e "${CYAN}"
    echo "╔═══════════════════════════════════════════════════════════════════╗"
    echo "║   🔄 BIZINO AI DEV v3.1 - Auto Update                             ║"
    echo "╚═══════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
    echo ""
}

check_git() {
    if ! command -v git &> /dev/null; then
        echo -e "${RED}❌ Git is not installed. Please install git first.${NC}"
        exit 1
    fi
}

get_token() {
    if [ -n "$_EMBEDDED_TOKEN" ]; then
        GITHUB_TOKEN="$_EMBEDDED_TOKEN"
        return 0
    fi
    return 1
}

load_config() {
    if [ -f "$CONFIG_FILE" ]; then
        source "$CONFIG_FILE"
        if [ -n "$PLATFORM" ]; then
            return 0
        fi
    fi
    return 1
}

clone_or_update_repo() {
    echo -e "${MAGENTA}[1/4] Fetching latest version...${NC}"
    
    # Construct authenticated URL
    if [ -n "$GITHUB_TOKEN" ]; then
        AUTH_URL="https://${GITHUB_TOKEN}@github.com/bizino/Bizino-Dev_Kit.git"
    else
        AUTH_URL="$REPO_URL"
    fi
    
    mkdir -p "$CACHE_DIR"
    
    if [ -d "$CACHE_DIR/.git" ]; then
        cd "$CACHE_DIR"
        git remote set-url origin "$AUTH_URL" 2>/dev/null || true
        git fetch origin main --quiet 2>/dev/null
        
        LOCAL=$(git rev-parse HEAD 2>/dev/null)
        REMOTE=$(git rev-parse origin/main 2>/dev/null)
        
        if [ "$LOCAL" = "$REMOTE" ]; then
            echo -e "${GREEN}  ✓ Already up to date!${NC}"
        else
            echo -e "${YELLOW}  ↓ New version available, updating...${NC}"
            git reset --hard origin/main --quiet
            echo -e "${GREEN}  ✓ Updated to latest version${NC}"
        fi
    else
        echo -e "${YELLOW}  ↓ First time setup, cloning repository...${NC}"
        rm -rf "$CACHE_DIR"
        git clone --depth 1 "$AUTH_URL" "$CACHE_DIR" --quiet 2>/dev/null
        if [ $? -ne 0 ]; then
            echo -e "${RED}  ✗ Clone failed. Check your internet connection.${NC}"
            exit 1
        fi
        echo -e "${GREEN}  ✓ Repository cloned${NC}"
    fi
    
    cd "$CACHE_DIR"
    CURRENT_COMMIT=$(git rev-parse --short HEAD 2>/dev/null || echo "unknown")
    CURRENT_DATE=$(git log -1 --format=%ci 2>/dev/null | cut -d' ' -f1 || echo "unknown")
    echo -e "${CYAN}  Version: ${WHITE}$CURRENT_COMMIT${CYAN} ($CURRENT_DATE)${NC}"
    echo ""
}

update_platform() {
    local PLATFORM=$1
    local PLATFORM_SRC=""
    local CONFIG_DIR=""
    local GLOBAL_WORKFLOWS_DIR=""
    local GLOBAL_SKILLS_DIR=""
    local GLOBAL_AGENTS_DIR=""
    local GLOBAL_ROLES_DIR=""
    local CONFIG_SOURCE=""
    local CONFIG_GLOBAL=""
    
    case $PLATFORM in
        "antigravity")
            echo -e "${CYAN}  → Antigravity / Gemini${NC}"
            PLATFORM_SRC="$CACHE_DIR/antigravity"
            CONFIG_DIR=".agent"
            GLOBAL_WORKFLOWS_DIR="$GEMINI_GLOBAL_WORKFLOWS"
            GLOBAL_SKILLS_DIR="$HOME/.gemini/antigravity/skills"
            GLOBAL_AGENTS_DIR="$HOME/.gemini/antigravity/agents"
            GLOBAL_ROLES_DIR="$HOME/.gemini/antigravity/roles"
            CONFIG_SOURCE="$PLATFORM_SRC/GEMINI.md"
            CONFIG_GLOBAL="$HOME/.gemini/GEMINI.md"
            ;;
        "claudekit")
            echo -e "${CYAN}  → Claude Code${NC}"
            PLATFORM_SRC="$CACHE_DIR/claudekit"
            CONFIG_DIR=".claude"
            GLOBAL_WORKFLOWS_DIR="$CLAUDE_GLOBAL_COMMANDS"
            GLOBAL_SKILLS_DIR="$HOME/.claude/skills"
            GLOBAL_AGENTS_DIR="$HOME/.claude/agents"
            GLOBAL_ROLES_DIR=""  # Claude doesn't use roles
            CONFIG_SOURCE="$PLATFORM_SRC/CLAUDE.md"
            CONFIG_GLOBAL="$CLAUDE_GLOBAL_CONFIG/CLAUDE.md"
            mkdir -p "$CLAUDE_GLOBAL_CONFIG"
            ;;
        "vscode-copilot")
            echo -e "${CYAN}  → VS Code Copilot${NC}"
            PLATFORM_SRC="$CACHE_DIR/vscode-copilot"
            CONFIG_DIR=".agent"
            GLOBAL_WORKFLOWS_DIR="$VSCODE_GLOBAL_DIR"
            GLOBAL_SKILLS_DIR="$HOME/.vscode-copilot/skills"
            GLOBAL_AGENTS_DIR="$HOME/.vscode-copilot/agents"
            GLOBAL_ROLES_DIR="$HOME/.vscode-copilot/roles"
            CONFIG_SOURCE="$PLATFORM_SRC/GEMINI.md"
            CONFIG_GLOBAL="$HOME/.vscode-copilot/GEMINI.md"
            ;;
    esac
    
    # Update workflows
    local WORKFLOWS_SRC="$PLATFORM_SRC/$CONFIG_DIR/workflows"
    if [ -d "$WORKFLOWS_SRC" ] && [ -n "$GLOBAL_WORKFLOWS_DIR" ]; then
        mkdir -p "$GLOBAL_WORKFLOWS_DIR"
        cp "$WORKFLOWS_SRC/"*.md "$GLOBAL_WORKFLOWS_DIR/" 2>/dev/null || true
        local WF_COUNT=$(ls "$GLOBAL_WORKFLOWS_DIR/"*.md 2>/dev/null | wc -l | tr -d ' ')
        echo -e "${GREEN}     ✓ Workflows: $WF_COUNT items${NC}"
    fi
    
    # Update skills (full rsync for incremental updates)
    local SKILLS_SRC="$PLATFORM_SRC/$CONFIG_DIR/skills"
    # VS Code Copilot has skills at root level
    [ "$PLATFORM" = "vscode-copilot" ] && SKILLS_SRC="$PLATFORM_SRC/skills"
    if [ -d "$SKILLS_SRC" ] && [ -n "$GLOBAL_SKILLS_DIR" ]; then
        mkdir -p "$GLOBAL_SKILLS_DIR"
        if command -v rsync &> /dev/null; then
            rsync -a --delete "$SKILLS_SRC/" "$GLOBAL_SKILLS_DIR/" 2>/dev/null
        else
            rm -rf "$GLOBAL_SKILLS_DIR"/*
            cp -R "$SKILLS_SRC/"* "$GLOBAL_SKILLS_DIR/" 2>/dev/null || true
        fi
        local SK_COUNT=$(ls -d "$GLOBAL_SKILLS_DIR"/*/ 2>/dev/null | wc -l | tr -d ' ')
        echo -e "${GREEN}     ✓ Skills: $SK_COUNT items${NC}"
    fi
    
    # Update agents
    local AGENTS_SRC="$PLATFORM_SRC/$CONFIG_DIR/agents"
    if [ -d "$AGENTS_SRC" ] && [ -n "$GLOBAL_AGENTS_DIR" ]; then
        mkdir -p "$GLOBAL_AGENTS_DIR"
        if command -v rsync &> /dev/null; then
            rsync -a --delete "$AGENTS_SRC/" "$GLOBAL_AGENTS_DIR/" 2>/dev/null
        else
            cp -R "$AGENTS_SRC/"* "$GLOBAL_AGENTS_DIR/" 2>/dev/null || true
        fi
        local AG_COUNT=$(ls "$GLOBAL_AGENTS_DIR/"*.md 2>/dev/null | wc -l | tr -d ' ')
        echo -e "${GREEN}     ✓ Agents: $AG_COUNT items${NC}"
    fi
    
    # Update roles
    local ROLES_SRC="$PLATFORM_SRC/$CONFIG_DIR/roles"
    if [ -d "$ROLES_SRC" ] && [ -n "$GLOBAL_ROLES_DIR" ]; then
        mkdir -p "$GLOBAL_ROLES_DIR"
        if command -v rsync &> /dev/null; then
            rsync -a --delete "$ROLES_SRC/" "$GLOBAL_ROLES_DIR/" 2>/dev/null
        else
            cp -R "$ROLES_SRC/"* "$GLOBAL_ROLES_DIR/" 2>/dev/null || true
        fi
        local RL_COUNT=$(ls "$GLOBAL_ROLES_DIR/"*.md 2>/dev/null | wc -l | tr -d ' ')
        echo -e "${GREEN}     ✓ Roles: $RL_COUNT items${NC}"
    fi
    
    # Update main config file (GEMINI.md / CLAUDE.md)
    if [ -f "$CONFIG_SOURCE" ] && [ -n "$CONFIG_GLOBAL" ]; then
        mkdir -p "$(dirname "$CONFIG_GLOBAL")"
        cp "$CONFIG_SOURCE" "$CONFIG_GLOBAL" 2>/dev/null || true
        echo -e "${GREEN}     ✓ Config: $(basename "$CONFIG_GLOBAL")${NC}"
    fi
}

# Update project directory (local .agent/ or .claude/)
update_project_directory() {
    local PLATFORM=$1
    local PROJECT_DIR="${2:-$(pwd)}"
    local UPDATED=false
    local PLATFORM_SRC=""
    local CONFIG_DIR=""
    
    case $PLATFORM in
        "antigravity")
            PLATFORM_SRC="$CACHE_DIR/antigravity"
            CONFIG_DIR=".agent"
            ;;
        "vscode-copilot")
            PLATFORM_SRC="$CACHE_DIR/vscode-copilot"
            CONFIG_DIR=".agent"
            ;;
        "claudekit")
            PLATFORM_SRC="$CACHE_DIR/claudekit"
            CONFIG_DIR=".claude"
            ;;
    esac
    
    local TARGET_CONFIG_DIR="$PROJECT_DIR/$CONFIG_DIR"
    
    # Check if project has the config directory
    if [ ! -d "$TARGET_CONFIG_DIR" ]; then
        echo -e "${YELLOW}  ⚠ No project detected in current directory${NC}"
        echo -e "${YELLOW}    (Run from a project with $CONFIG_DIR/ to update project files)${NC}"
        return
    fi
    
    echo -e "${CYAN}  📁 Project: $PROJECT_DIR${NC}"
    
    # Update workflows
    local WORKFLOWS_SRC="$PLATFORM_SRC/$CONFIG_DIR/workflows"
    if [ -d "$WORKFLOWS_SRC" ] && [ -d "$TARGET_CONFIG_DIR/workflows" ]; then
        cp "$WORKFLOWS_SRC/"*.md "$TARGET_CONFIG_DIR/workflows/" 2>/dev/null || true
        local WF_COUNT=$(ls "$TARGET_CONFIG_DIR/workflows/"*.md 2>/dev/null | wc -l | tr -d ' ')
        echo -e "${GREEN}     ✓ Workflows: $WF_COUNT items${NC}"
        UPDATED=true
    fi
    
    # Update skills
    local SKILLS_SRC="$PLATFORM_SRC/$CONFIG_DIR/skills"
    [ "$PLATFORM" = "vscode-copilot" ] && SKILLS_SRC="$PLATFORM_SRC/skills"
    if [ -d "$SKILLS_SRC" ] && [ -d "$TARGET_CONFIG_DIR/skills" ]; then
        if command -v rsync &> /dev/null; then
            rsync -a --delete "$SKILLS_SRC/" "$TARGET_CONFIG_DIR/skills/" 2>/dev/null
        else
            rm -rf "$TARGET_CONFIG_DIR/skills"/*
            cp -R "$SKILLS_SRC/"* "$TARGET_CONFIG_DIR/skills/" 2>/dev/null || true
        fi
        local SK_COUNT=$(ls -d "$TARGET_CONFIG_DIR/skills"/*/ 2>/dev/null | wc -l | tr -d ' ')
        echo -e "${GREEN}     ✓ Skills: $SK_COUNT items${NC}"
        UPDATED=true
    fi
    
    # Update agents
    local AGENTS_SRC="$PLATFORM_SRC/$CONFIG_DIR/agents"
    if [ -d "$AGENTS_SRC" ] && [ -d "$TARGET_CONFIG_DIR/agents" ]; then
        if command -v rsync &> /dev/null; then
            rsync -a --delete "$AGENTS_SRC/" "$TARGET_CONFIG_DIR/agents/" 2>/dev/null
        else
            cp -R "$AGENTS_SRC/"* "$TARGET_CONFIG_DIR/agents/" 2>/dev/null || true
        fi
        local AG_COUNT=$(ls "$TARGET_CONFIG_DIR/agents/"*.md 2>/dev/null | wc -l | tr -d ' ')
        echo -e "${GREEN}     ✓ Agents: $AG_COUNT items${NC}"
        UPDATED=true
    fi
    
    # Update roles
    local ROLES_SRC="$PLATFORM_SRC/$CONFIG_DIR/roles"
    if [ -d "$ROLES_SRC" ] && [ -d "$TARGET_CONFIG_DIR/roles" ]; then
        if command -v rsync &> /dev/null; then
            rsync -a --delete "$ROLES_SRC/" "$TARGET_CONFIG_DIR/roles/" 2>/dev/null
        else
            cp -R "$ROLES_SRC/"* "$TARGET_CONFIG_DIR/roles/" 2>/dev/null || true
        fi
        local RL_COUNT=$(ls "$TARGET_CONFIG_DIR/roles/"*.md 2>/dev/null | wc -l | tr -d ' ')
        echo -e "${GREEN}     ✓ Roles: $RL_COUNT items${NC}"
        UPDATED=true
    fi
    
    # Update main config file (GEMINI.md / CLAUDE.md)
    local CONFIG_NAME="GEMINI.md"
    [ "$PLATFORM" = "claudekit" ] && CONFIG_NAME="CLAUDE.md"
    if [ -f "$PLATFORM_SRC/$CONFIG_NAME" ] && [ -f "$PROJECT_DIR/$CONFIG_NAME" ]; then
        cp "$PLATFORM_SRC/$CONFIG_NAME" "$PROJECT_DIR/$CONFIG_NAME" 2>/dev/null || true
        echo -e "${GREEN}     ✓ $CONFIG_NAME${NC}"
        UPDATED=true
    fi
    
    # Update project update.sh if exists
    if [ -f "$CACHE_DIR/update.sh" ] && [ -f "$PROJECT_DIR/update.sh" ]; then
        cp "$CACHE_DIR/update.sh" "$PROJECT_DIR/update.sh" 2>/dev/null || true
        chmod +x "$PROJECT_DIR/update.sh" 2>/dev/null || true
        echo -e "${GREEN}     ✓ update.sh${NC}"
    fi
    
    if [ "$UPDATED" = true ]; then
        echo -e "${GREEN}     ✓ Project fully updated!${NC}"
    fi
}

# ==============================================================================
# Main
# ==============================================================================

show_header
check_git

# Save original directory BEFORE any cd operations
ORIGINAL_DIR="$(pwd)"

# Get token
if get_token; then
    echo -e "${GREEN}✓ Authentication ready${NC}"
else
    echo -e "${YELLOW}⚠️ Using public access${NC}"
fi
echo ""

# Clone/Update repository
clone_or_update_repo

# ==============================================================================
# Load saved platform or ask
# ==============================================================================
echo -e "${MAGENTA}[2/4] Checking configuration...${NC}"

if load_config; then
    echo -e "${GREEN}  ✓ Found saved platform: ${WHITE}$PLATFORM${NC}"
    
    case $PLATFORM in
        "antigravity") PLATFORMS=("antigravity") ;;
        "claudekit") PLATFORMS=("claudekit") ;;
        "vscode-copilot") PLATFORMS=("vscode-copilot") ;;
        "all") PLATFORMS=("antigravity" "claudekit" "vscode-copilot") ;;
        *)
            echo -e "${RED}  ✗ Unknown platform: $PLATFORM${NC}"
            exit 1
            ;;
    esac
else
    echo -e "${YELLOW}  ⚠ No saved configuration found.${NC}"
    echo -e "${YELLOW}  Please run install.sh first to select your platform.${NC}"
    echo ""
    echo -e "${WHITE}  Or choose now:${NC}"
    echo ""
    echo -e "  ${GREEN}1)${NC} ⚡ Antigravity / Gemini"
    echo -e "  ${GREEN}2)${NC} 🤖 Claude Code"
    echo -e "  ${GREEN}3)${NC} 💻 VS Code Copilot"
    echo -e "  ${GREEN}4)${NC} 🎯 All Platforms"
    echo ""
    read -p "$(echo -e ${YELLOW}Chọn [1-4]: ${NC})" PLATFORM_CHOICE
    
    case $PLATFORM_CHOICE in
        1) 
            PLATFORMS=("antigravity")
            PLATFORM="antigravity"
            ;;
        2) 
            PLATFORMS=("claudekit")
            PLATFORM="claudekit"
            ;;
        3) 
            PLATFORMS=("vscode-copilot")
            PLATFORM="vscode-copilot"
            ;;
        4) 
            PLATFORMS=("antigravity" "claudekit" "vscode-copilot")
            PLATFORM="all"
            ;;
        *)
            echo -e "${RED}Invalid option. Exiting.${NC}"
            exit 1
            ;;
    esac
    
    # Save the choice
    mkdir -p "$BIZINO_HOME"
    echo "PLATFORM=$PLATFORM" > "$CONFIG_FILE"
    echo "INSTALLED_DATE=$(date +%Y-%m-%d)" >> "$CONFIG_FILE"
    echo -e "${GREEN}  ✓ Platform saved for future updates${NC}"
fi

echo ""
echo -e "${MAGENTA}[3/4] Updating Global Files...${NC}"

for PLATFORM in "${PLATFORMS[@]}"; do
    update_platform "$PLATFORM"
done

# Update the update script itself
cp "$CACHE_DIR/update.sh" "$BIZINO_HOME/update.sh" 2>/dev/null || true
chmod +x "$BIZINO_HOME/update.sh" 2>/dev/null || true

echo ""
echo -e "${MAGENTA}[4/4] Updating Project Directory...${NC}"

# Use ORIGINAL_DIR saved at script start (before any cd operations)
for PLATFORM in "${PLATFORMS[@]}"; do
    update_project_directory "$PLATFORM" "$ORIGINAL_DIR"
done

echo ""

# ==============================================================================
# VERIFICATION - Check if files were actually updated
# ==============================================================================
echo -e "${MAGENTA}[VERIFY] Kiểm tra cập nhật...${NC}"
VERIFY_PASSED=true
VERIFY_ERRORS=()

verify_dir() {
    local dir="$1"
    local desc="$2"
    local min_count="${3:-1}"
    if [ -d "$dir" ]; then
        local count=$(ls "$dir" 2>/dev/null | wc -l | tr -d ' ')
        if [ "$count" -ge "$min_count" ]; then
            echo -e "  ${GREEN}✓${NC} $desc ($count items)"
            return 0
        else
            echo -e "  ${YELLOW}⚠${NC} $desc ($count items, expected >=$min_count)"
            return 1
        fi
    else
        echo -e "  ${RED}✗${NC} $desc - MISSING!"
        VERIFY_ERRORS+=("$desc")
        VERIFY_PASSED=false
        return 1
    fi
}

verify_file() {
    local file="$1"
    local desc="$2"
    if [ -f "$file" ]; then
        local size=$(ls -lh "$file" 2>/dev/null | awk '{print $5}')
        echo -e "  ${GREEN}✓${NC} $desc (${size})"
        return 0
    else
        echo -e "  ${RED}✗${NC} $desc - MISSING!"
        VERIFY_ERRORS+=("$desc")
        VERIFY_PASSED=false
        return 1
    fi
}

echo ""
for P in "${PLATFORMS[@]}"; do
    case $P in
        "antigravity")
            echo -e "${CYAN}Antigravity / Gemini:${NC}"
            verify_dir "$GEMINI_GLOBAL_WORKFLOWS" "Global Workflows" 10
            verify_dir "$HOME/.gemini/antigravity/skills" "Global Skills" 40
            verify_file "$HOME/.gemini/GEMINI.md" "Global GEMINI.md"
            # v3.1 check: react-best-practices
            verify_dir "$HOME/.gemini/antigravity/skills/react-best-practices/rules" "react-best-practices (v3.1)" 40
            ;;
        "claudekit")
            echo -e "${CYAN}Claude Code:${NC}"
            verify_dir "$CLAUDE_GLOBAL_COMMANDS" "Global Commands" 10
            verify_dir "$HOME/.claude/skills" "Global Skills" 40
            verify_file "$CLAUDE_GLOBAL_CONFIG/CLAUDE.md" "Global CLAUDE.md"
            verify_dir "$HOME/.claude/skills/react-best-practices/rules" "react-best-practices (v3.1)" 40
            ;;
        "vscode-copilot")
            echo -e "${CYAN}VS Code Copilot:${NC}"
            verify_dir "$VSCODE_GLOBAL_DIR" "Global Workflows" 10
            verify_dir "$HOME/.vscode-copilot/skills" "Global Skills" 40
            verify_file "$HOME/.vscode-copilot/GEMINI.md" "Global GEMINI.md"
            verify_dir "$HOME/.vscode-copilot/skills/react-best-practices/rules" "react-best-practices (v3.1)" 40
            ;;
    esac
    echo ""
done

verify_file "$BIZINO_HOME/update.sh" "update.sh (self-update)"
echo ""

if [ "$VERIFY_PASSED" = true ]; then
    echo -e "${GREEN}✅ Verification PASSED - Update successful!${NC}"
else
    echo -e "${RED}⚠️ Verification FAILED - Some items missing:${NC}"
    for err in "${VERIFY_ERRORS[@]}"; do
        echo -e "   ${RED}• $err${NC}"
    done
fi
echo ""

# ==============================================================================
# Summary
# ==============================================================================
echo -e "${GREEN}╔═══════════════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║      ✅ UPDATE COMPLETE!                                          ║${NC}"
echo -e "${GREEN}╚═══════════════════════════════════════════════════════════════════╝${NC}"
echo ""

echo -e "${WHITE}📦 Updated Platforms:${NC}"
for P in "${PLATFORMS[@]}"; do
    echo -e "   ${GREEN}✓${NC} $P"
done
echo ""

echo -e "${WHITE}📊 Version:${NC}"
echo -e "   ${CYAN}$CURRENT_COMMIT ($CURRENT_DATE)${NC}"
echo ""

echo -e "${WHITE}💡 Next update:${NC}"
echo -e "   ${CYAN}~/.bizino-dev-kit/update.sh${NC}"
echo -e "   ${CYAN}(Your platform choice is saved, no need to select again!)${NC}"
echo ""

echo -e "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo -e "${WHITE}  Bizino AI DEV v3.1 - Premium Software Development, Automated${NC}"
echo -e "${MAGENTA}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
echo ""
