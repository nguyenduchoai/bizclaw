#!/bin/bash
# Skills Installation Script for Linux/macOS
# Installs all dependencies for Claude Code skills
#
# Exit codes (rustup model):
#   0 = Success (full or partial)
#   1 = Fatal error (no Python, broken venv)
#   2 = Partial success (some optional deps failed)

# set -e removed - we handle errors per-phase

# Parse command line arguments
SKIP_CONFIRM=false
WITH_SUDO=false
RESUME_MODE=false
RETRY_FAILED=false
while [[ "$#" -gt 0 ]]; do
    case $1 in
        -y|--yes) SKIP_CONFIRM=true ;;
        --with-sudo) WITH_SUDO=true ;;
        --resume) RESUME_MODE=true ;;
        --retry-failed) RETRY_FAILED=true ;;
        *) echo "Unknown parameter: $1"; exit 1 ;;
    esac
    shift
done

# Check for NON_INTERACTIVE environment variable
if [[ -n "${NON_INTERACTIVE}" ]]; then
    SKIP_CONFIRM=true
fi

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Configuration
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
VENV_DIR="$SCRIPT_DIR/.venv"
STATE_FILE="$SCRIPT_DIR/.install-state.json"
LOG_DIR="$VENV_DIR/logs"

# Installation tracking arrays (Bash 3.2+ compatible - indexed arrays only)
declare -a INSTALLED_CRITICAL=()
declare -a INSTALLED_OPTIONAL=()
declare -a FAILED_OPTIONAL=()
declare -a SKIPPED_SUDO=()
FINAL_EXIT_CODE=0

# Detect OS
detect_os() {
    if [[ "$OSTYPE" == "darwin"* ]]; then
        echo "macos"
    elif [[ "$OSTYPE" == "linux-gnu"* ]]; then
        echo "linux"
    else
        echo "unknown"
    fi
}

# Print functions (must be defined before check_bash_version)
print_header() {
    echo -e "\n${BLUE}===================================================${NC}"
    echo -e "${BLUE}$1${NC}"
    echo -e "${BLUE}===================================================${NC}\n"
}

print_success() {
    echo -e "${GREEN}✓${NC} $1"
}

print_warning() {
    echo -e "${YELLOW}⚠${NC} $1"
}

print_error() {
    echo -e "${RED}✗${NC} $1"
}

print_info() {
    echo -e "${BLUE}ℹ${NC} $1"
}

# Check Bash version (3.2+ required for compatibility)
check_bash_version() {
    # Get major version number
    bash_major="${BASH_VERSINFO[0]}"

    if [ "$bash_major" -lt 3 ]; then
        print_error "Bash 3.0+ required (found Bash $BASH_VERSION)"
        print_info "Please upgrade Bash and re-run this script"
        exit 1
    fi

    if [ "$bash_major" -lt 4 ]; then
        print_warning "Bash 3.x detected (Bash 4+ recommended)"

        if [[ "$OSTYPE" == "darwin"* ]]; then
            print_info "macOS ships with Bash 3.2 by default"
            print_info "For better compatibility: brew install bash"
            print_info "Then use: /usr/local/bin/bash install.sh"
        fi

        print_info "Continuing with compatibility mode..."
    fi
}

OS=$(detect_os)
check_bash_version

# Check if command exists
command_exists() {
    command -v "$1" >/dev/null 2>&1
}

# ============================================================================
# Installation Tracking Functions
# ============================================================================

track_success() {
    local category="$1" name="$2"
    if [[ "$category" == "critical" ]]; then
        INSTALLED_CRITICAL+=("$name")
    else
        INSTALLED_OPTIONAL+=("$name")
    fi
}

track_failure() {
    local category="$1" name="$2" reason="$3"
    if [[ "$category" == "critical" ]]; then
        FINAL_EXIT_CODE=1
    else
        FAILED_OPTIONAL+=("$name: $reason")
        [[ "$FINAL_EXIT_CODE" == "0" ]] && FINAL_EXIT_CODE=2
    fi
}

track_skipped() {
    local name="$1" reason="$2"
    SKIPPED_SUDO+=("$name: $reason")
}

# ============================================================================
# State Persistence Functions
# ============================================================================

# Initialize or load state
init_state() {
    if [[ "$RESUME_MODE" == "true" ]] && [[ -f "$STATE_FILE" ]]; then
        print_info "Resuming from previous installation..."
        return 0
    fi

    # Create fresh state
    cat > "$STATE_FILE" << EOF
{
  "version": 1,
  "started_at": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "last_updated": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "phases": {
    "system_deps": "pending",
    "node_deps": "pending",
    "python_env": "pending",
    "verify": "pending"
  },
  "packages": {
    "installed": [],
    "failed": [],
    "skipped": []
  }
}
EOF
}

# Update phase status
update_phase() {
    local phase="$1" status="$2"
    if [[ ! -f "$STATE_FILE" ]]; then
        return 0
    fi
    local temp_file="$STATE_FILE.tmp"

    # Simple sed replacement (jq not required)
    sed "s/\"$phase\": \"[^\"]*\"/\"$phase\": \"$status\"/" "$STATE_FILE" > "$temp_file"
    mv "$temp_file" "$STATE_FILE"

    # Update timestamp
    sed "s/\"last_updated\": \"[^\"]*\"/\"last_updated\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"/" "$STATE_FILE" > "$temp_file"
    mv "$temp_file" "$STATE_FILE"
}

# Check if phase is already done (for resume)
phase_done() {
    local phase="$1"
    grep -q "\"$phase\": \"done\"" "$STATE_FILE" 2>/dev/null
}

# Clean state file (on complete success)
clean_state() {
    if [[ -f "$STATE_FILE" ]]; then
        rm -f "$STATE_FILE"
        print_info "Installation state cleaned (success)"
    fi
}

# ============================================================================
# Build Tools Detection
# ============================================================================

# Check if system has C build tools for compiling Python packages
has_build_tools() {
    if command_exists gcc || command_exists clang; then
        if [[ "$OS" == "linux" ]]; then
            # Check for Python dev headers
            if [[ -f /usr/include/python3*/Python.h ]] || \
               python3-config --includes &>/dev/null; then
                return 0
            fi
        elif [[ "$OS" == "macos" ]]; then
            # macOS: Xcode command line tools include headers
            if xcode-select -p &>/dev/null; then
                return 0
            fi
        fi
    fi
    return 1
}

# Try pip install with wheel-first fallback
# Returns: 0=success, 1=failed
try_pip_install() {
    local package_spec="$1"
    local log_file="$2"
    local package_name="${package_spec%%[=<>]*}"  # Strip version specifier

    # Phase 1: Try with prefer-binary (wheels first)
    if pip install "$package_spec" --prefer-binary 2>&1 | tee -a "$log_file"; then
        return 0
    fi

    # Phase 2: Check if we can build from source
    if ! has_build_tools; then
        print_warning "$package_name: No wheel available, no build tools"
        if [[ "$OS" == "linux" ]]; then
            print_info "Install build tools: sudo apt-get install gcc python3-dev"
        elif [[ "$OS" == "macos" ]]; then
            print_info "Install build tools: xcode-select --install"
        fi
        return 1
    fi

    # Phase 3: Try source build
    print_info "Trying source build for $package_name..."
    if pip install "$package_spec" --no-binary "$package_name" 2>&1 | tee -a "$log_file"; then
        print_success "$package_name installed (source build)"
        return 0
    fi

    print_error "$package_name: Both wheel and source build failed"
    return 1
}

# Install system package - no prompts, just install or skip
# CLI controls sudo via --with-sudo flag
install_system_package() {
    local package_name="$1"
    local display_name="$2"
    local check_commands="$3"  # Comma-separated commands to check

    # Check if already installed (check multiple commands)
    IFS=',' read -ra cmds <<< "$check_commands"
    for cmd in "${cmds[@]}"; do
        if command_exists "$cmd"; then
            print_success "$display_name already installed"
            track_success "optional" "$display_name"
            return 0
        fi
    done

    # macOS: brew doesn't need sudo
    if [[ "$OS" == "macos" ]]; then
        print_info "Installing $display_name..."
        if brew install "$package_name" 2>/dev/null; then
            print_success "$display_name installed"
            track_success "optional" "$display_name"
            return 0
        else
            print_warning "$display_name: brew install failed"
            track_failure "optional" "$display_name" "brew install failed"
            return 1
        fi
    fi

    # Linux: only install if --with-sudo was passed
    if [[ "$WITH_SUDO" == "true" ]]; then
        print_info "Installing $display_name (sudo)..."
        if sudo apt-get install -y "$package_name"; then
            print_success "$display_name installed"
            track_success "optional" "$display_name"
            return 0
        else
            print_warning "$display_name: apt-get install failed"
            track_failure "optional" "$display_name" "apt-get install failed"
            return 1
        fi
    else
        # No sudo permission - track as skipped
        print_info "$display_name: skipped (no --with-sudo)"
        track_skipped "$display_name" "requires sudo"
        return 0  # Don't fail, just skip
    fi
}

# Check and install system package manager
check_package_manager() {
    if [[ "$OS" == "macos" ]]; then
        if ! command_exists brew; then
            print_warning "Homebrew not found. Installing Homebrew..."
            /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
            print_success "Homebrew installed"
        else
            print_success "Homebrew found"
        fi
    elif [[ "$OS" == "linux" ]]; then
        if command_exists apt-get; then
            print_success "apt-get found"
        elif command_exists yum; then
            print_success "yum found"
        else
            print_warning "No supported package manager found (apt-get or yum)"
            print_info "System packages will be skipped"
            # Don't exit - just warn and continue
        fi
    fi
}

# Install system dependencies
install_system_deps() {
    print_header "Installing System Dependencies"

    # Update apt cache if we have sudo permission (Linux only)
    if [[ "$OS" == "linux" ]] && [[ "$WITH_SUDO" == "true" ]]; then
        print_info "Updating package lists..."
        sudo apt-get update -qq
    fi

    # FFmpeg (required for media-processing skill)
    install_system_package "ffmpeg" "FFmpeg" "ffmpeg"

    # ImageMagick (required for media-processing skill)
    install_system_package "imagemagick" "ImageMagick" "magick,convert"

    # PostgreSQL client (optional - just check)
    if command_exists psql; then
        print_success "PostgreSQL client already installed"
    fi

    # Docker (optional - just check)
    if command_exists docker; then
        print_success "Docker already installed ($(docker --version))"
    fi
}

# Install Node.js and npm packages
install_node_deps() {
    print_header "Installing Node.js Dependencies"

    # Check Node.js
    if command_exists node; then
        NODE_VERSION=$(node --version)
        print_success "Node.js already installed ($NODE_VERSION)"
    else
        print_info "Installing Node.js..."
        if [[ "$OS" == "macos" ]]; then
            brew install node
        elif [[ "$OS" == "linux" ]]; then
            curl -fsSL https://deb.nodesource.com/setup_20.x | sudo -E bash -
            sudo apt-get install -y nodejs
        fi
        print_success "Node.js installed"
    fi

    # Install global npm packages
    print_info "Installing global npm packages..."

    # Package name to CLI command mapping (some packages have different CLI names)
    # Using indexed array with colon-separated pairs for Bash 3.2+ compatibility
    npm_packages=(
        "rmbg-cli:rmbg"
        "pnpm:pnpm"
        "wrangler:wrangler"
        "repomix:repomix"
    )

    for package_pair in "${npm_packages[@]}"; do
        # Split "package:command" on colon
        IFS=':' read -r package cmd <<< "$package_pair"

        # Check CLI command first (handles standalone installs like brew, curl, etc.)
        if command_exists "$cmd"; then
            version=$("$cmd" --version 2>&1 | head -n1 || echo "available")
            print_success "$package already installed ($version)"
        # Fallback: check if installed via npm registry
        elif npm list -g "$package" >/dev/null 2>&1; then
            print_success "$package already installed via npm"
        else
            print_info "Installing $package..."
            npm install -g "$package" 2>/dev/null || {
                print_warning "Failed to install $package globally. Trying with sudo..."
                sudo npm install -g "$package"
            }
            print_success "$package installed"
        fi
    done

    # Install local npm packages for skills
    print_info "Installing local npm packages for skills..."

    # chrome-devtools
    if [ -d "$SCRIPT_DIR/chrome-devtools/scripts" ] && [ -f "$SCRIPT_DIR/chrome-devtools/scripts/package.json" ]; then
        print_info "Installing chrome-devtools dependencies..."
        (cd "$SCRIPT_DIR/chrome-devtools/scripts" && npm install --quiet)
        print_success "chrome-devtools dependencies installed"
    fi

    # sequential-thinking
    if [ -d "$SCRIPT_DIR/sequential-thinking" ] && [ -f "$SCRIPT_DIR/sequential-thinking/package.json" ]; then
        print_info "Installing sequential-thinking dependencies..."
        (cd "$SCRIPT_DIR/sequential-thinking" && npm install --quiet)
        print_success "sequential-thinking dependencies installed"
    fi

    # mcp-management
    if [ -d "$SCRIPT_DIR/mcp-management/scripts" ] && [ -f "$SCRIPT_DIR/mcp-management/scripts/package.json" ]; then
        print_info "Installing mcp-management dependencies..."
        (cd "$SCRIPT_DIR/mcp-management/scripts" && npm install --quiet)
        print_success "mcp-management dependencies installed"
    fi

    # markdown-novel-viewer (marked, highlight.js, gray-matter)
    if [ -d "$SCRIPT_DIR/markdown-novel-viewer" ] && [ -f "$SCRIPT_DIR/markdown-novel-viewer/package.json" ]; then
        print_info "Installing markdown-novel-viewer dependencies..."
        (cd "$SCRIPT_DIR/markdown-novel-viewer" && npm install --quiet)
        print_success "markdown-novel-viewer dependencies installed"
    fi

    # plans-kanban (gray-matter)
    if [ -d "$SCRIPT_DIR/plans-kanban" ] && [ -f "$SCRIPT_DIR/plans-kanban/package.json" ]; then
        print_info "Installing plans-kanban dependencies..."
        (cd "$SCRIPT_DIR/plans-kanban" && npm install --quiet)
        print_success "plans-kanban dependencies installed"
    fi

    # Optional: Shopify CLI (ask user unless auto-confirming)
    if [ -d "$SCRIPT_DIR/shopify" ]; then
        if [[ "$SKIP_CONFIRM" == "true" ]]; then
            print_info "Skipping Shopify CLI installation (optional, use --yes to install all)"
        else
            read -p "Install Shopify CLI for Shopify skill? (y/N) " -n 1 -r
            echo
            if [[ $REPLY =~ ^[Yy]$ ]]; then
                print_info "Installing Shopify CLI..."
                npm install -g @shopify/cli @shopify/theme 2>/dev/null || {
                    print_warning "Failed to install Shopify CLI globally. Trying with sudo..."
                    sudo npm install -g @shopify/cli @shopify/theme
                }
                print_success "Shopify CLI installed"
            fi
        fi
    fi
}

# Setup Python virtual environment
setup_python_env() {
    print_header "Setting Up Python Environment"

    # Track successful and failed installations
    local successful_skills=()
    local failed_skills=()

    # Check Python
    if command_exists python3; then
        PYTHON_VERSION=$(python3 --version)
        PYTHON_PATH=$(which python3)
        print_success "Python3 found ($PYTHON_VERSION)"

        # Check for broken UV Python installation
        if [[ "$PYTHON_PATH" == *"/.local/share/uv/"* ]]; then
            # Verify UV Python works by testing venv creation
            if ! python3 -c "import sys; sys.exit(0 if '/install' not in sys.base_prefix else 1)" 2>/dev/null; then
                print_error "UV Python installation is broken (corrupted sys.base_prefix)"
                print_info "Please reinstall Python using Homebrew:"
                print_info "  brew install python@3.12"
                print_info "  export PATH=\"/opt/homebrew/bin:\$PATH\""
                print_info "Or fix UV Python:"
                print_info "  uv python uninstall 3.12"
                print_info "  uv python install 3.12"
                exit 1
            fi
        fi
    else
        print_error "Python3 not found. Please install Python 3.7+"
        exit 1
    fi

    # Create virtual environment (or recreate if corrupted)
    create_venv() {
        # Try normal venv creation first
        if python3 -m venv "$VENV_DIR" 2>/dev/null; then
            return 0
        fi

        # If ensurepip fails (common on macOS), create without pip and bootstrap manually
        print_warning "Standard venv creation failed, trying without ensurepip..."
        if python3 -m venv --without-pip "$VENV_DIR"; then
            # Bootstrap pip manually with error handling
            source "$VENV_DIR/bin/activate"
            if ! curl -sS https://bootstrap.pypa.io/get-pip.py | python3; then
                print_error "Failed to bootstrap pip (network issue or get-pip.py failed)"
                deactivate
                rm -rf "$VENV_DIR"
                return 1
            fi
            deactivate
            return 0
        fi

        return 1
    }

    if [ -d "$VENV_DIR" ]; then
        # Verify venv is valid by checking for activate script AND python executable
        if [ -f "$VENV_DIR/bin/activate" ] && [ -x "$VENV_DIR/bin/python3" ]; then
            print_success "Virtual environment already exists at $VENV_DIR"
        else
            print_warning "Virtual environment is corrupted (missing activate or python3). Recreating..."
            rm -rf "$VENV_DIR"
            if create_venv; then
                print_success "Virtual environment recreated"
            else
                print_error "Failed to create virtual environment"
                exit 1
            fi
        fi
    else
        print_info "Creating virtual environment at $VENV_DIR..."
        if create_venv; then
            print_success "Virtual environment created"
        else
            print_error "Failed to create virtual environment"
            exit 1
        fi
    fi

    # Activate and install packages
    print_info "Activating virtual environment..."
    source "$VENV_DIR/bin/activate"

    # Create log directory
    local LOG_DIR="$VENV_DIR/logs"
    mkdir -p "$LOG_DIR"

    # Upgrade pip with logging (use --prefer-binary)
    print_info "Upgrading pip..."
    if pip install --upgrade pip --prefer-binary 2>&1 | tee "$LOG_DIR/pip-upgrade.log" | tail -n 3; then
        print_success "pip upgraded successfully"
    else
        print_warning "pip upgrade failed (continuing anyway)"
        print_info "See log: $LOG_DIR/pip-upgrade.log"
    fi

    # Install dependencies from all skills' requirements.txt files
    print_info "Installing Python dependencies from all skills..."

    local installed_count=0
    for skill_dir in "$SCRIPT_DIR"/*; do
        if [ -d "$skill_dir" ]; then
            skill_name=$(basename "$skill_dir")

            # Skip .venv and document-skills
            if [ "$skill_name" == ".venv" ] || [ "$skill_name" == "document-skills" ]; then
                continue
            fi

            # Install main requirements.txt with wheel-first approach
            if [ -f "$skill_dir/scripts/requirements.txt" ]; then
                local SKILL_LOG="$LOG_DIR/install-${skill_name}.log"

                print_info "Installing $skill_name dependencies..."

                # Read requirements and install one-by-one for granular tracking
                local pkg_success=0
                local pkg_fail=0
                while IFS= read -r line || [[ -n "$line" ]]; do
                    # Skip comments and empty lines
                    [[ "$line" =~ ^#.*$ ]] && continue
                    [[ -z "${line// }" ]] && continue

                    # Strip inline comments (e.g., "package>=1.0  # comment" -> "package>=1.0")
                    line="${line%%#*}"
                    line="${line%"${line##*[![:space:]]}"}"  # trim trailing whitespace
                    [[ -z "$line" ]] && continue

                    if try_pip_install "$line" "$SKILL_LOG"; then
                        pkg_success=$((pkg_success + 1))
                    else
                        pkg_fail=$((pkg_fail + 1))
                        track_failure "optional" "$skill_name:$line" "Package install failed"
                    fi
                done < "$skill_dir/scripts/requirements.txt"

                if [[ $pkg_fail -eq 0 ]]; then
                    print_success "$skill_name: all $pkg_success packages installed"
                    track_success "optional" "$skill_name"
                    successful_skills+=("$skill_name")
                    installed_count=$((installed_count + 1))
                else
                    print_warning "$skill_name: $pkg_success installed, $pkg_fail failed"
                    failed_skills+=("$skill_name")
                fi
            fi

            # Install test requirements.txt
            if [ -f "$skill_dir/scripts/tests/requirements.txt" ]; then
                local SKILL_TEST_LOG="$LOG_DIR/install-${skill_name}-tests.log"

                print_info "Installing $skill_name test dependencies..."

                if pip install -r "$skill_dir/scripts/tests/requirements.txt" --prefer-binary 2>&1 | tee "$SKILL_TEST_LOG"; then
                    print_success "$skill_name test dependencies installed successfully"
                else
                    print_warning "$skill_name test dependencies failed to install"
                    # Don't fail installation if test deps fail (less critical)
                fi
            fi
        fi
    done

    # Install .claude/scripts requirements (contains pyyaml for generate_catalogs.py)
    local SCRIPTS_REQ="$SCRIPT_DIR/../scripts/requirements.txt"
    if [ -f "$SCRIPTS_REQ" ]; then
        local SCRIPTS_LOG="$LOG_DIR/install-scripts.log"
        print_info "Installing .claude/scripts dependencies..."

        local pkg_success=0
        local pkg_fail=0
        while IFS= read -r line || [[ -n "$line" ]]; do
            [[ "$line" =~ ^#.*$ ]] && continue
            [[ -z "${line// }" ]] && continue
            line="${line%%#*}"
            line="${line%"${line##*[![:space:]]}"}"
            [[ -z "$line" ]] && continue

            if try_pip_install "$line" "$SCRIPTS_LOG"; then
                pkg_success=$((pkg_success + 1))
            else
                pkg_fail=$((pkg_fail + 1))
                track_failure "optional" "scripts:$line" "Package install failed"
            fi
        done < "$SCRIPTS_REQ"

        if [[ $pkg_fail -eq 0 ]]; then
            print_success ".claude/scripts: all $pkg_success packages installed"
            track_success "optional" "scripts"
        else
            print_warning ".claude/scripts: $pkg_success installed, $pkg_fail failed"
        fi
    fi

    # Print installation summary (brief - final report comes later)
    print_header "Python Dependencies Installation Summary"

    if [ ${#successful_skills[@]} -gt 0 ]; then
        print_success "Successfully installed ${#successful_skills[@]} skill(s)"
    fi

    if [ ${#failed_skills[@]} -gt 0 ]; then
        print_warning "${#failed_skills[@]} skill(s) had package failures (see final report)"
        # Don't exit 1 - track failures instead and continue
    elif [ ${#successful_skills[@]} -eq 0 ]; then
        print_warning "No skill requirements.txt files found"
    else
        print_success "All Python dependencies installed successfully"
    fi

    deactivate
}

# Verify installations
verify_installations() {
    print_header "Verifying Installations"

    # FFmpeg
    if command_exists ffmpeg; then
        print_success "FFmpeg is available"
    else
        print_warning "FFmpeg is not available"
    fi

    # ImageMagick (check both magick and convert - older versions use convert)
    if command_exists magick || command_exists convert; then
        print_success "ImageMagick is available"
    else
        print_warning "ImageMagick is not available"
    fi

    # Node.js & npm
    if command_exists node; then
        print_success "Node.js is available"
    else
        print_warning "Node.js is not available"
    fi

    if command_exists npm; then
        print_success "npm is available"
    else
        print_warning "npm is not available"
    fi

    declare -a npm_packages=(
        "rmbg"
        "pnpm"
        "wrangler"
        "repomix"
    )

    for package in "${npm_packages[@]}"; do
        if command_exists "$package"; then
            print_success "$package CLI is available"
        else
            print_warning "$package CLI is not available"
        fi
    done

    # Check Python packages
    if [ -d "$VENV_DIR" ]; then
        source "$VENV_DIR/bin/activate"
        if python -c "import google.genai" 2>/dev/null; then
            print_success "google-genai Python package is available"
        else
            print_warning "google-genai Python package is not available"
        fi
        deactivate
    fi
}

# ============================================================================
# Final Report Functions
# ============================================================================

generate_remediation_commands() {
    local has_sudo_skipped=false
    local has_python_failed=false

    # Check if we have sudo-skipped packages
    if [[ ${#SKIPPED_SUDO[@]} -gt 0 ]]; then
        has_sudo_skipped=true
    fi

    # Check if we have Python package failures
    if [[ ${#FAILED_OPTIONAL[@]} -gt 0 ]]; then
        has_python_failed=true
    fi

    if [[ "$has_sudo_skipped" == "false" ]] && [[ "$has_python_failed" == "false" ]]; then
        return 0
    fi

    echo ""
    echo -e "${BLUE}---------------------------------------------------${NC}"
    echo -e "${BLUE}Manual Installation Commands:${NC}"
    echo -e "${BLUE}---------------------------------------------------${NC}"
    echo ""

    if [[ "$has_sudo_skipped" == "true" ]]; then
        echo "# System packages (requires sudo):"
        echo "sudo apt-get update"
        for item in "${SKIPPED_SUDO[@]}"; do
            local pkg="${item%%:*}"
            case "$pkg" in
                FFmpeg) echo "sudo apt-get install -y ffmpeg" ;;
                ImageMagick) echo "sudo apt-get install -y imagemagick" ;;
                *) echo "# $pkg: see documentation" ;;
            esac
        done
        echo ""
    fi

    if [[ "$has_python_failed" == "true" ]]; then
        echo "# Python packages (may require build tools):"
        if [[ "$OS" == "linux" ]]; then
            echo "sudo apt-get install -y gcc python3-dev libjpeg-dev zlib1g-dev"
        elif [[ "$OS" == "macos" ]]; then
            echo "xcode-select --install"
            echo "brew install jpeg libpng"
        fi
        echo "source $VENV_DIR/bin/activate"

        for item in "${FAILED_OPTIONAL[@]}"; do
            local pkg="${item%%:*}"
            # Extract package name from skill:package format
            if [[ "$pkg" == *":"* ]]; then
                pkg="${pkg#*:}"
            fi
            echo "pip install $pkg"
        done
        echo ""
    fi
}

print_final_report() {
    echo ""
    echo -e "${BLUE}===================================================${NC}"
    echo -e "${BLUE}           Installation Report${NC}"
    echo -e "${BLUE}===================================================${NC}"
    echo ""

    # Installed section
    local installed_count=$((${#INSTALLED_CRITICAL[@]} + ${#INSTALLED_OPTIONAL[@]}))
    if [[ $installed_count -gt 0 ]]; then
        echo -e "${GREEN}Installed ($installed_count):${NC}"
        for item in "${INSTALLED_CRITICAL[@]}"; do
            echo -e "  ${GREEN}✓${NC} $item"
        done
        for item in "${INSTALLED_OPTIONAL[@]}"; do
            echo -e "  ${GREEN}✓${NC} $item"
        done
        echo ""
    fi

    # Skipped section
    if [[ ${#SKIPPED_SUDO[@]} -gt 0 ]]; then
        echo -e "${YELLOW}Skipped (${#SKIPPED_SUDO[@]}):${NC}"
        for item in "${SKIPPED_SUDO[@]}"; do
            local name="${item%%:*}"
            local reason="${item#*:}"
            echo -e "  ${YELLOW}~${NC} $name (${reason# })"
        done
        echo ""
    fi

    # Degraded/Failed section
    if [[ ${#FAILED_OPTIONAL[@]} -gt 0 ]]; then
        echo -e "${RED}Degraded (${#FAILED_OPTIONAL[@]}):${NC}"
        for item in "${FAILED_OPTIONAL[@]}"; do
            local name="${item%%:*}"
            local reason="${item#*:}"
            echo -e "  ${RED}!${NC} $name (${reason# })"
        done
        echo ""
    fi

    # Remediation commands
    generate_remediation_commands

    # Exit status line
    echo -e "${BLUE}===================================================${NC}"
    case $FINAL_EXIT_CODE in
        0) echo -e " ${GREEN}Exit: 0 (success - all dependencies installed)${NC}" ;;
        1) echo -e " ${RED}Exit: 1 (failed - critical dependencies missing)${NC}" ;;
        2) echo -e " ${YELLOW}Exit: 2 (partial - some optional deps failed)${NC}" ;;
    esac
    echo -e "${BLUE}===================================================${NC}"
    echo ""
}

# Escape string for JSON (handle all special chars)
json_escape() {
    printf '%s' "$1" | sed 's/\\/\\\\/g; s/"/\\"/g' | tr '\n' ' ' | tr '\r' ' ' | tr '\t' ' '
}

# Write structured error summary for CLI to parse
write_error_summary() {
    # Only write if there are failures
    if [[ $FINAL_EXIT_CODE -eq 0 ]]; then
        return 0
    fi

    # Write to a file that CLI can read
    local summary_file="$SCRIPT_DIR/.install-error-summary.json"

    # Build JSON arrays carefully for bash compatibility
    local critical_json="[]"
    local optional_json="[]"
    local skipped_json="[]"

    if [[ ${#FAILED_OPTIONAL[@]} -gt 0 ]]; then
        optional_json="["
        local first=true
        for item in "${FAILED_OPTIONAL[@]}"; do
            if [[ "$first" == "true" ]]; then
                first=false
            else
                optional_json+=","
            fi
            optional_json+="\"$(json_escape "$item")\""
        done
        optional_json+="]"
    fi

    if [[ ${#SKIPPED_SUDO[@]} -gt 0 ]]; then
        skipped_json="["
        local first=true
        for item in "${SKIPPED_SUDO[@]}"; do
            if [[ "$first" == "true" ]]; then
                first=false
            else
                skipped_json+=","
            fi
            skipped_json+="\"$(json_escape "$item")\""
        done
        skipped_json+="]"
    fi

    cat > "$summary_file" << EOF
{
  "exit_code": $FINAL_EXIT_CODE,
  "timestamp": "$(date -Iseconds 2>/dev/null || date -u +%Y-%m-%dT%H:%M:%SZ)",
  "critical_failures": $critical_json,
  "optional_failures": $optional_json,
  "skipped": $skipped_json,
  "remediation": {
    "sudo_packages": "sudo apt-get install -y ffmpeg imagemagick",
    "build_tools": "sudo apt-get install -y gcc python3-dev libjpeg-dev zlib1g-dev",
    "pip_retry": "source $VENV_DIR/bin/activate && pip install <package>"
  }
}
EOF

    print_info "Error summary written to: $summary_file"
}

# Print usage instructions (now just brief tips)
print_usage() {
    echo -e "${GREEN}To use the Python virtual environment:${NC}"
    echo -e "  source .claude/skills/.venv/bin/activate"
    echo ""
    echo -e "${BLUE}For more information, see:${NC}"
    echo -e "  .claude/skills/INSTALLATION.md"
    echo ""
}

# Main installation flow
main() {
    echo ""  # Just add spacing, don't clear terminal
    print_header "Claude Code Skills Installation"
    print_info "OS: $OS"
    print_info "Script directory: $SCRIPT_DIR"
    if [[ "$WITH_SUDO" == "true" ]]; then
        print_info "Mode: with sudo (--with-sudo)"
    else
        print_info "Mode: without sudo (system packages will be skipped)"
    fi
    if [[ "$RESUME_MODE" == "true" ]]; then
        print_info "Mode: resuming previous installation"
    fi
    echo ""

    if [[ "$OS" == "unknown" ]]; then
        print_error "Unsupported operating system"
        exit 1
    fi

    # Confirm installation (skip if --yes flag or NON_INTERACTIVE env is set)
    if [[ "$SKIP_CONFIRM" == "false" ]]; then
        read -p "This will install system packages and Node.js dependencies. Continue? (y/N) " -n 1 -r
        echo
        if [[ ! $REPLY =~ ^[Yy]$ ]]; then
            print_warning "Installation cancelled"
            exit 0
        fi
    else
        print_info "Auto-confirming installation (--yes flag or NON_INTERACTIVE mode)"
    fi

    # Initialize state tracking
    init_state

    # Phase 1: System deps
    if phase_done "system_deps"; then
        print_success "System deps: already processed (resume)"
    else
        update_phase "system_deps" "running"
        check_package_manager
        install_system_deps
        update_phase "system_deps" "done"
    fi

    # Phase 2: Node deps
    if phase_done "node_deps"; then
        print_success "Node deps: already installed (resume)"
    else
        update_phase "node_deps" "running"
        install_node_deps
        update_phase "node_deps" "done"
    fi

    # Phase 3: Python env
    if phase_done "python_env"; then
        print_success "Python env: already set up (resume)"
    else
        update_phase "python_env" "running"
        setup_python_env
        update_phase "python_env" "done"
    fi

    # Phase 4: Verify
    update_phase "verify" "running"
    verify_installations
    update_phase "verify" "done"

    # Print final report with all tracking info
    print_final_report
    print_usage

    # Write error summary for CLI to parse
    write_error_summary

    # Clean state on complete success
    if [[ "$FINAL_EXIT_CODE" -eq 0 ]]; then
        clean_state
    fi

    exit $FINAL_EXIT_CODE
}

# Run main function
main
