#!/bin/bash
#
# vibes-ubuntu-v3.sh
# Interactive rainbow menu installer for dev environment components on Ubuntu 24.04
#
# This is a safer/reliable variant of vibes-ubuntu.sh.
# - Does NOT overwrite the original.
# - Improves detection to match the *target user* (not root).
# - Adds missing prereqs (curl/ca-certificates/gnupg).
# - Uses pipefail for safer curl|bash flows.
#
# Usage: sudo bash vibes-ubuntu-v3.sh
# Or: chmod +x vibes-ubuntu-v3.sh && sudo ./vibes-ubuntu-v3.sh
#

set -euo pipefail
IFS=$'\n\t'

export DEBIAN_FRONTEND=noninteractive

# Color palette for rainbow effect
RAINBOW_COLORS=(
    '\033[0;31m'  # Red
    '\033[0;33m'  # Yellow
    '\033[0;32m'  # Green
    '\033[0;36m'  # Cyan
    '\033[0;34m'  # Blue
    '\033[0;35m'  # Magenta
    '\033[0;91m'  # Light Red
    '\033[0;93m'  # Light Yellow
    '\033[0;92m'  # Light Green
    '\033[0;96m'  # Light Cyan
)

# Standard colors
NC='\033[0m'       # No Color
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[0;33m'
CYAN='\033[0;36m'
GRAY='\033[0;90m'
WHITE='\033[1;37m'
MAGENTA='\033[0;35m'

# Terminal formatting
BOLD='\033[1m'
DIM='\033[2m'
REVERSE='\033[7m'

# Global state
CURRENT_USER="${SUDO_USER:-$USER}"
USER_HOME=$(eval echo ~"${CURRENT_USER}")

# Check if running as root
if [ "${EUID:-0}" -ne 0 ]; then
    echo -e "${RED}This script must be run as root (use sudo)${NC}"
    exit 1
fi

# Ensure we have the actual user, not root
if [ -z "${SUDO_USER:-}" ]; then
    echo -e "${YELLOW}Warning: SUDO_USER not set. Attempting to detect user...${NC}"
    CURRENT_USER=$(who | awk '{print $1}' | sort -u | head -n1)
    if [ -z "${CURRENT_USER}" ] || [ "${CURRENT_USER}" = "root" ]; then
        echo -e "${RED}Cannot determine non-root user. Please run with sudo.${NC}"
        exit 1
    fi
    USER_HOME=$(eval echo ~"${CURRENT_USER}")
fi

echo -e "${CYAN}Running as root for: ${CURRENT_USER} (home: ${USER_HOME})${NC}"

# Utility functions (must be defined before first use)
write_info() { echo -e "${CYAN}-> $1${NC}"; }
write_ok() { echo -e "${GREEN}OK: $1${NC}"; }
write_warn() { echo -e "${YELLOW}!! $1${NC}"; }
write_error() { echo -e "${RED}ERROR: $1${NC}"; }

# Resolve the user's Desktop directory (supports XDG user-dirs)
resolve_desktop_dir() {
    local desktop_dir="${USER_HOME}/Desktop"
    local xdg_file="${USER_HOME}/.config/user-dirs.dirs"

    if [ -f "$xdg_file" ]; then
        local xdg_val
        xdg_val=$(grep -E '^XDG_DESKTOP_DIR=' "$xdg_file" 2>/dev/null | head -n1 || true)
        if [ -n "$xdg_val" ]; then
            xdg_val=${xdg_val#XDG_DESKTOP_DIR=}
            xdg_val=${xdg_val%\"}
            xdg_val=${xdg_val#\"}
            xdg_val=${xdg_val//\$HOME/${USER_HOME}}
            if [ -n "$xdg_val" ]; then
                desktop_dir="$xdg_val"
            fi
        fi
    fi

    echo "$desktop_dir"
}

DESKTOP_DIR="$(resolve_desktop_dir)"
mkdir -p "$DESKTOP_DIR" 2>/dev/null || true

# Logging (tee everything to a Desktop log file)
LOG_FILE="${DESKTOP_DIR}/vibes-ubuntu-$(date +%Y%m%d-%H%M%S).log"
touch "$LOG_FILE" 2>/dev/null || true
exec > >(tee -a "$LOG_FILE") 2>&1

# Ensure log ends up readable/owned by the target user
cleanup() {
    local user_group
    user_group=$(id -gn "$CURRENT_USER" 2>/dev/null || echo "$CURRENT_USER")
    chown "$CURRENT_USER":"$user_group" "$LOG_FILE" 2>/dev/null || true
}
trap cleanup EXIT

write_quick_start_file() {
    local quick_start_file="${DESKTOP_DIR}/Vibe-Coding-Quick-Start.txt"
    local user_group
    user_group=$(id -gn "$CURRENT_USER" 2>/dev/null || echo "$CURRENT_USER")

    cat > "$quick_start_file" <<'EOF'
Vibe Coding Quick Start

Open a NEW terminal window after the installer finishes so PATH updates are visible.

1. Qwen Code
   qwen
   Follow the browser sign-in flow, then type /quit when you are done.

2. Gemini CLI
   gemini
   Follow the browser sign-in flow, then type /quit when you are done.

3. VS Code + AI extensions
   Open Visual Studio Code from your app launcher.
   Open Cline or Kilo Code from the left sidebar.
   Choose Qwen / Qwen Code as the provider if you want a free starting point.

Other useful commands:
   aicp
   codex
   claude

If one of the commands is not found right away, log out and back in once so PATH updates fully apply.
EOF

    chown "$CURRENT_USER":"$user_group" "$quick_start_file" 2>/dev/null || true
    write_ok "Created desktop quick-start guide: $quick_start_file"
}

write_info "Logging to: $LOG_FILE"

# Execute command with retries (accepts a single command string)
execute_with_retry() {
    local max_attempts=3
    local attempt=1
    local command="$*"

    while [ "$attempt" -le "$max_attempts" ]; do
        if eval "$command"; then
            return 0
        fi

        write_warn "Attempt $attempt/$max_attempts failed for: $command"
        if [ "$attempt" -lt "$max_attempts" ]; then
            sleep 2
            attempt=$((attempt + 1))
        else
            return 1
        fi
    done
}

# Test if command exists in *current* PATH
test_command() {
    command -v "$1" >/dev/null 2>&1
}

# Build a conservative PATH for the target user (so we can see ~/.local/bin, snap bins, etc.)
user_path() {
    echo "${USER_HOME}/.local/bin:/snap/bin:/usr/local/bin:/usr/bin:/bin"
}

# Run a command as the target (non-root) user, with HOME and PATH set.
# Uses bash -lc so user PATH additions in profile files can be picked up when present.
run_as_user() {
    local cmd="$*"
    sudo -H -u "$CURRENT_USER" env "HOME=$USER_HOME" "PATH=$(user_path)" bash -lc "$cmd"
}

# Test if a command exists for the target user
user_has_command() {
    local cmd="$1"
    run_as_user "command -v '$cmd' >/dev/null 2>&1"
}

# Update package lists with retry
update_apt() {
    write_info "Updating apt package lists..."
    execute_with_retry "apt-get update -qq" || {
        write_warn "apt-get update failed, trying with --fix-missing"
        apt-get update --fix-missing -qq || true
    }
}

# Install package with retry
install_apt_package() {
    local package="$1"
    local attempt=1
    local max_attempts=3

    write_info "Installing $package via apt..."

    while [ "$attempt" -le "$max_attempts" ]; do
        if apt-get install -y "$package"; then
            write_ok "$package installed successfully"
            return 0
        fi

        write_warn "Attempt $attempt/$max_attempts failed"
        if [ "$attempt" -lt "$max_attempts" ]; then
            sleep 2
            apt-get update -qq || true
            attempt=$((attempt + 1))
        else
            write_error "Failed to install $package after $max_attempts attempts"
            return 1
        fi
    done
}

# Ensure baseline prereqs commonly needed by installers
ensure_prereqs() {
    update_apt
    install_apt_package ca-certificates || true
    install_apt_package curl || true
    install_apt_package gnupg || true
}

# Install Git
install_git() {
    if user_has_command git; then
        write_ok "Git already present"
        return 0
    fi

    update_apt
    install_apt_package git || {
        write_warn "Trying alternative: git-core"
        install_apt_package git-core || return 1
    }

    run_as_user "git --version" || true
    write_ok "Git installed"
}

# Install Node.js LTS
install_nodejs() {
    if user_has_command node; then
        write_ok "Node.js already present ($(run_as_user 'node --version' 2>/dev/null || true))"
        return 0
    fi

    write_info "Installing Node.js LTS..."
    ensure_prereqs

    # Method 1: Try NodeSource repository (recommended for latest LTS)
    if ! user_has_command node; then
        write_info "Method 1: Trying NodeSource repository..."
        if execute_with_retry "curl -fsSL https://deb.nodesource.com/setup_lts.x | bash -"; then
            install_apt_package nodejs || true
        fi
    fi

    # Method 2: Try snap if NodeSource failed
    if ! user_has_command node; then
        write_info "Method 2: Trying snap..."
        if test_command snap; then
            execute_with_retry "snap install node --classic --channel=lts/stable" || true
        fi
    fi

    # Method 3: Try Ubuntu repository as last resort
    if ! user_has_command node; then
        write_info "Method 3: Trying Ubuntu repository..."
        update_apt
        install_apt_package nodejs || true
        install_apt_package npm || true
    fi

    # Verify installation
    if user_has_command node; then
        run_as_user "node --version" || true
        write_ok "Node.js installed"

        # Upgrade npm (best-effort)
        if user_has_command npm; then
            write_info "Upgrading npm..."
            run_as_user "npm install -g npm@latest" >/dev/null 2>&1 || true
        fi
        return 0
    fi

    write_error "Failed to install Node.js"
    return 1
}

# Resolve Python executable for root/system (Ubuntu standard)
resolve_python() {
    if test_command python3; then
        echo "python3"
        return 0
    fi
    if test_command python; then
        echo "python"
        return 0
    fi
    return 1
}

# Install Python stack (pip, pipx, uv) for the target user
install_python_stack() {
    local python_cmd
    python_cmd=$(resolve_python) || python_cmd=""

    if [ -z "$python_cmd" ]; then
        write_info "Installing Python 3..."
        update_apt

        if apt-cache show python3.12 >/dev/null 2>&1; then
            install_apt_package python3.12 || true
            install_apt_package python3.12-venv || true
            install_apt_package python3.12-dev || true
        else
            install_apt_package python3 || true
            install_apt_package python3-venv || true
            install_apt_package python3-dev || true
        fi

        install_apt_package python3-pip || true

        if ! test_command python && test_command python3; then
            ln -sf /usr/bin/python3 /usr/local/bin/python 2>/dev/null || true
        fi

        python_cmd=$(resolve_python) || {
            write_error "Failed to install Python"
            return 1
        }
    else
        write_ok "Python already present ($($python_cmd --version 2>/dev/null || true))"
    fi

    write_info "Using Python: $python_cmd"

    ensure_prereqs

    # Ensure pip is available
    write_info "Ensuring pip is available..."
    if ! "$python_cmd" -m pip --version >/dev/null 2>&1; then
        install_apt_package python3-pip || true
        if ! "$python_cmd" -m pip --version >/dev/null 2>&1; then
            write_info "Installing pip via get-pip.py..."
            local tmp_pip="/tmp/get-pip.py"
            if curl -fsSL https://bootstrap.pypa.io/get-pip.py -o "$tmp_pip"; then
                run_as_user "'$python_cmd' '$tmp_pip' --user" || true
                rm -f "$tmp_pip"
            fi
        fi
    fi

    # Upgrade pip (as user)
    write_info "Upgrading pip..."
    run_as_user "'$python_cmd' -m pip install --upgrade pip --user" >/dev/null 2>&1 || true

    # Install pipx (prefer apt, fallback to user pip)
    write_info "Installing pipx..."
    if ! user_has_command pipx; then
        if ! install_apt_package pipx; then
            run_as_user "'$python_cmd' -m pip install --user --upgrade pipx" || true
        fi
        # Ensure pipx is visible and PATH is configured
        run_as_user "'$python_cmd' -m pipx ensurepath" >/dev/null 2>&1 || true
    fi

    # Install uv (user) — MUST use the official installer (Astral)
    write_info "Installing uv (official installer)..."
    if ! user_has_command uv; then
        # Ensure curl is present (prereqs already tries, but keep this resilient)
        if ! test_command curl; then
            install_apt_package curl || true
        fi

        if test_command curl; then
            run_as_user "curl -LsSf https://astral.sh/uv/install.sh | sh" || true
        elif test_command wget; then
            run_as_user "wget -qO- https://astral.sh/uv/install.sh | sh" || true
        else
            write_warn "Cannot install uv: need curl or wget"
        fi
    fi

    # Verify installations from the user's perspective
    local pip_ok=false
    local pipx_ok=false
    local uv_ok=false

    run_as_user "'$python_cmd' -m pip --version >/dev/null 2>&1" && pip_ok=true
    user_has_command pipx && pipx_ok=true
    user_has_command uv && uv_ok=true

    if $pip_ok && $pipx_ok && $uv_ok; then
        write_ok "Python stack (pip, pipx, uv) installed"
        return 0
    fi

    write_warn "Python stack partially installed (pip:$pip_ok pipx:$pipx_ok uv:$uv_ok)"
    return 0
}

# Install aicodeprep-gui
install_aicodeprep() {
    if user_has_command aicodeprep-gui; then
        write_ok "aicodeprep-gui already present"
        return 0
    fi

    install_python_stack || return 1

    local python_cmd
    python_cmd=$(resolve_python) || {
        write_error "Python not found"
        return 1
    }

    write_info "Installing aicodeprep-gui..."

    # Prefer pipx for isolation; fall back to --user pip
    if user_has_command pipx; then
        run_as_user "pipx install aicodeprep-gui" || {
            write_warn "pipx install failed, trying python -m pipx..."
            run_as_user "'$python_cmd' -m pipx install aicodeprep-gui" || true
        }
    else
        run_as_user "'$python_cmd' -m pip install --user aicodeprep-gui" || true
    fi

    if user_has_command aicodeprep-gui; then
        write_ok "aicodeprep-gui installed"
        return 0
    fi

    write_warn "aicodeprep-gui installed but not in PATH (try logging out/in)"
    return 0
}

# Install VS Code
install_vscode() {
    if user_has_command code; then
        write_ok "VS Code already present"
        return 0
    fi

    write_info "Installing VS Code..."
    ensure_prereqs

    # Method 1: Microsoft repository
    if ! user_has_command code; then
        write_info "Method 1: Trying Microsoft repository..."

        install_apt_package wget || true
        install_apt_package apt-transport-https || true
        install_apt_package gpg || true

        wget -qO- https://packages.microsoft.com/keys/microsoft.asc | gpg --dearmor > /tmp/packages.microsoft.gpg
        install -D -o root -g root -m 644 /tmp/packages.microsoft.gpg /etc/apt/keyrings/packages.microsoft.gpg
        rm -f /tmp/packages.microsoft.gpg

        echo "deb [arch=amd64,arm64,armhf signed-by=/etc/apt/keyrings/packages.microsoft.gpg] https://packages.microsoft.com/repos/code stable main" > /etc/apt/sources.list.d/vscode.list

        update_apt
        install_apt_package code || true
    fi

    # Method 2: Snap
    if ! user_has_command code && test_command snap; then
        write_info "Method 2: Trying snap..."
        snap install code --classic || true
    fi

    # Method 3: Direct .deb download
    if ! user_has_command code; then
        write_info "Method 3: Trying direct .deb download..."
        local deb_file="/tmp/vscode.deb"
        if test_command wget && wget -q "https://code.visualstudio.com/sha/download?build=stable&os=linux-deb-x64" -O "$deb_file"; then
            apt-get install -y "$deb_file" || dpkg -i "$deb_file" || true
            apt-get install -f -y || true
            rm -f "$deb_file"
        fi
    fi

    if user_has_command code; then
        write_ok "VS Code installed"
        return 0
    fi

    write_error "Failed to install VS Code"
    return 1
}

# Install VS Code extensions
install_vscode_extensions() {
    install_vscode || return 1

    if ! user_has_command code; then
        write_error "VS Code not available"
        return 1
    fi

    local extensions=(
        "saoudrizwan.claude-dev"
        "kilocode.Kilo-Code"
        "ms-vscode.live-server"
        "oderwat.indent-rainbow"
        "esbenp.prettier-vscode"
    )

    write_info "Installing VS Code extensions..."
    for ext in "${extensions[@]}"; do
        write_info "Installing extension: $ext"
        run_as_user "code --install-extension '$ext' --force" || {
            write_warn "Failed to install $ext, retrying..."
            sleep 2
            run_as_user "code --install-extension '$ext' --force" || true
        }
    done

    write_ok "VS Code extensions installation complete"
}

# Install npm global package
install_npm_global() {
    local package="$1"
    local command="$2"

    # If it's available for the user already, don't reinstall
    if user_has_command "$command"; then
        write_ok "$package already present"
        return 0
    fi

    install_nodejs || return 1

    if ! user_has_command npm && ! test_command npm; then
        write_error "npm not available"
        return 1
    fi

    write_info "Installing $package..."

    # Try as current user first
    if run_as_user "npm install -g '$package'" >/dev/null 2>&1; then
        write_ok "$package installed"
        return 0
    fi

    # Fallback: install as root (common when npm global prefix is system-owned)
    write_info "Installing $package as root..."
    npm install -g "$package" >/dev/null 2>&1 || {
        write_error "Failed to install $package"
        return 1
    }

    write_ok "$package installed"
}

install_qwen_code() { install_npm_global "@qwen-code/qwen-code" "qwen"; }
install_gemini_cli() { install_npm_global "@google/gemini-cli" "gemini"; }
install_codex_cli() { install_npm_global "@openai/codex" "codex"; }

# Claude Code CLI name differs across platforms/versions (often 'claude')
detect_claude_code() {
    if user_has_command claude; then
        echo "installed:true|details:command:claude"
    elif user_has_command claude-code; then
        echo "installed:true|details:command:claude-code"
    else
        echo "installed:false|details:"
    fi
}

install_claude_code_cli() {
    if user_has_command claude || user_has_command claude-code; then
        write_ok "@anthropic-ai/claude-code already present"
        return 0
    fi

    # Install package; command may be 'claude' or 'claude-code' afterwards.
    install_npm_global "@anthropic-ai/claude-code" "claude" || true

    if user_has_command claude || user_has_command claude-code; then
        write_ok "@anthropic-ai/claude-code installed"
        return 0
    fi

    write_warn "@anthropic-ai/claude-code installed but command not found yet (try a new terminal/session)"
    return 0
}

# Detection helpers: report status for the target user, not root.
detect_git() {
    if user_has_command git; then
        echo "installed:true|details:$(run_as_user 'git --version' 2>/dev/null | head -1)"
    else
        echo "installed:false|details:"
    fi
}

detect_nodejs() {
    if user_has_command node; then
        echo "installed:true|details:$(run_as_user 'node --version' 2>/dev/null | head -1)"
    else
        echo "installed:false|details:"
    fi
}

detect_python() {
    local python_cmd
    python_cmd=$(resolve_python) || {
        echo "installed:false|details:"
        return
    }

    local pip_ok=false
    local pipx_ok=false
    local uv_ok=false

    run_as_user "'$python_cmd' -m pip --version >/dev/null 2>&1" && pip_ok=true
    user_has_command pipx && pipx_ok=true
    user_has_command uv && uv_ok=true

    if $pip_ok && $pipx_ok && $uv_ok; then
        echo "installed:true|details:$(run_as_user \"'$python_cmd' --version\" 2>&1 | head -1)"
    else
        echo "installed:false|details:pip:$pip_ok pipx:$pipx_ok uv:$uv_ok"
    fi
}

detect_aicodeprep() {
    if user_has_command aicodeprep-gui; then
        echo "installed:true|details:"
    else
        echo "installed:false|details:"
    fi
}

detect_vscode() {
    if user_has_command code; then
        echo "installed:true|details:$(run_as_user 'code --version' 2>/dev/null | head -1)"
    else
        echo "installed:false|details:"
    fi
}

detect_vscode_extensions() {
    if ! user_has_command code; then
        echo "installed:false|details:"
        return
    fi

    local required=("saoudrizwan.claude-dev" "kilocode.Kilo-Code" "ms-vscode.live-server" "oderwat.indent-rainbow" "esbenp.prettier-vscode")
    local installed_list
    installed_list=$(run_as_user "code --list-extensions" 2>/dev/null || true)

    local count=0
    local missing=()
    for ext in "${required[@]}"; do
        if echo "$installed_list" | grep -q "^${ext}$"; then
            count=$((count + 1))
        else
            missing+=("$ext")
        fi
    done

    if [ "$count" -eq "${#required[@]}" ]; then
        echo "installed:true|details:${count}/${#required[@]} installed"
    else
        local missing_str
        missing_str=$(IFS=','; echo "${missing[*]}")
        echo "installed:false|details:${count}/${#required[@]} installed (missing: ${missing_str})"
    fi
}

detect_npm_command() {
    local cmd="$1"
    if user_has_command "$cmd"; then
        echo "installed:true|details:"
    else
        echo "installed:false|details:"
    fi
}

# Task definitions
declare -A TASKS
declare -A TASK_ORDER
TASK_COUNT=0

add_task() {
    local id="$1"
    local name="$2"
    local description="$3"
    local detect_func="$4"
    local install_func="$5"

    TASKS["${id}_name"]="$name"
    TASKS["${id}_desc"]="$description"
    TASKS["${id}_detect"]="$detect_func"
    TASKS["${id}_install"]="$install_func"
    TASKS["${id}_selected"]="true"
    TASK_ORDER[$TASK_COUNT]="$id"
    TASK_COUNT=$((TASK_COUNT + 1))
}

init_tasks() {
    add_task "git" "Git" "Source control client" "detect_git" "install_git"
    add_task "nodejs" "Node.js LTS + npm" "JavaScript runtime and package manager" "detect_nodejs" "install_nodejs"
    add_task "python" "Python 3 + pip/pipx/uv" "Python runtime and package tools" "detect_python" "install_python_stack"
    add_task "aicodeprep" "aicodeprep-gui (pipx)" "AI Code Prep GUI CLI" "detect_aicodeprep" "install_aicodeprep"
    add_task "vscode" "Visual Studio Code" "Primary code editor" "detect_vscode" "install_vscode"
    add_task "extensions" "VS Code Extensions (5 total)" "Claude Dev, Kilo-Code, Live Server, Indent Rainbow, Prettier" "detect_vscode_extensions" "install_vscode_extensions"
    add_task "qwen" "Global npm: @qwen-code/qwen-code" "Qwen code assistant CLI" "detect_npm_command qwen" "install_qwen_code"
    add_task "gemini" "Global npm: @google/gemini-cli" "Gemini CLI from Google" "detect_npm_command gemini" "install_gemini_cli"
    add_task "codex" "OpenAI Codex CLI" "OpenAI Codex CLI for AI-assisted coding" "detect_npm_command codex" "install_codex_cli"
    add_task "claude" "Claude Code CLI" "Claude Code from Anthropic" "detect_claude_code" "install_claude_code_cli"
}

update_task_detection() {
    for i in $(seq 0 $((TASK_COUNT - 1))); do
        local id="${TASK_ORDER[$i]}"
        local detect_func="${TASKS[${id}_detect]}"

        # detect_func may include arguments (e.g., "detect_npm_command qwen")
        local result
        result=$(eval "$detect_func")

        local installed
        installed=$(echo "$result" | grep -oP '(?<=installed:)[^|]+' || true)
        local details
        details=$(echo "$result" | grep -oP '(?<=details:).*' || true)

        TASKS["${id}_installed"]="$installed"
        TASKS["${id}_details"]="$details"
    done
}

render_menu() {
    local current_index=$1

    clear

    local title="Code with AI Helper / Vibe Coding Installer (Ubuntu 24.04)"
    for ((i=0; i<${#title}; i++)); do
        local color_idx=$((i % ${#RAINBOW_COLORS[@]}))
        echo -ne "${RAINBOW_COLORS[$color_idx]}${title:$i:1}${NC}"
    done
    echo ""
    echo -e "${GRAY}$(printf '%.0s-' {1..80})${NC}"
    echo ""

    for i in $(seq 0 $((TASK_COUNT - 1))); do
        local id="${TASK_ORDER[$i]}"
        local name="${TASKS[${id}_name]}"
        local installed="${TASKS[${id}_installed]}"
        local details="${TASKS[${id}_details]}"
        local selected="${TASKS[${id}_selected]}"

        local mark="[ ]"
        [ "$selected" = "true" ] && mark="[x]"

        local status="Available"
        [ "$installed" = "true" ] && status="Installed"

        local text="$mark $name - $status"
        [ -n "$details" ] && text="$text | $details"

        if [ "$i" -eq "$current_index" ]; then
            echo -e "${REVERSE}${WHITE}$text${NC}"
        else
            local color_idx=$((i % ${#RAINBOW_COLORS[@]}))
            echo -e "${RAINBOW_COLORS[$color_idx]}$text${NC}"
        fi
    done

    echo ""
    local current_id="${TASK_ORDER[$current_index]}"
    echo -e "${WHITE}Selected: ${TASKS[${current_id}_name]}${NC}"
    echo -e "${GRAY}${TASKS[${current_id}_desc]}${NC}"
    [ -n "${TASKS[${current_id}_details]}" ] && echo -e "${DIM}Status: ${TASKS[${current_id}_details]}${NC}"
    echo ""
    echo -e "${GRAY}Use arrows to move, Space to toggle, Enter to install, R to refresh, Q to quit.${NC}"
    echo ""
}

run_menu() {
    local current_index=0
    local menu_active=true

    while $menu_active; do
        render_menu "$current_index"

        read -rsn1 key

        if [ "$key" = $'\x1b' ]; then
            read -rsn2 -t 0.1 key2 || true
            case "$key2" in
                '[A')
                    current_index=$((current_index - 1))
                    [ "$current_index" -lt 0 ] && current_index=$((TASK_COUNT - 1))
                    continue
                    ;;
                '[B')
                    current_index=$((current_index + 1))
                    [ "$current_index" -ge "$TASK_COUNT" ] && current_index=0
                    continue
                    ;;
            esac
        fi

        case "$key" in
            " ")
                local id="${TASK_ORDER[$current_index]}"
                if [ "${TASKS[${id}_selected]}" = "true" ]; then
                    TASKS["${id}_selected"]="false"
                else
                    TASKS["${id}_selected"]="true"
                fi
                ;;
            r|R)
                write_info "Refreshing detection..."
                update_task_detection
                ;;
            q|Q)
                menu_active=false
                return 1
                ;;
            "")
                menu_active=false
                return 0
                ;;
        esac
    done
}

main() {
    echo -e "${CYAN}Vibe Code Dev Environment Installer for Ubuntu 24.04${NC}"
    echo ""

    init_tasks

    write_info "Detecting installed components (for user: $CURRENT_USER)..."
    update_task_detection

    if ! run_menu; then
        write_warn "Installation cancelled"
        exit 0
    fi

    local selected_ids=()
    for i in $(seq 0 $((TASK_COUNT - 1))); do
        local id="${TASK_ORDER[$i]}"
        if [ "${TASKS[${id}_selected]}" = "true" ]; then
            selected_ids+=("$id")
        fi
    done

    if [ ${#selected_ids[@]} -eq 0 ]; then
        write_warn "No components selected. Nothing to install."
        exit 0
    fi

    echo ""
    echo -e "${CYAN}Selected components:${NC}"
    for id in "${selected_ids[@]}"; do
        echo -e "${GRAY} - ${TASKS[${id}_name]}${NC}"
    done
    echo ""
    read -r -p "Press Enter to continue or Ctrl+C to cancel..."

    clear
    echo -e "${CYAN}Starting installation...${NC}"
    echo ""

    for id in "${selected_ids[@]}"; do
        echo ""
        write_info "Processing ${TASKS[${id}_name]}..."

        local install_func="${TASKS[${id}_install]}"
        if $install_func; then
            write_ok "${TASKS[${id}_name]} complete"
        else
            write_error "Failed to install ${TASKS[${id}_name]}"
        fi

        # Refresh detection after each install
        local detect_func="${TASKS[${id}_detect]}"
        local result
        result=$(eval "$detect_func")
        local installed
        installed=$(echo "$result" | grep -oP '(?<=installed:)[^|]+' || true)
        local details
        details=$(echo "$result" | grep -oP '(?<=details:).*' || true)
        TASKS["${id}_installed"]="$installed"
        TASKS["${id}_details"]="$details"
    done

    echo ""
    echo -e "${CYAN}═══════════════════════════════════════════════════════════════${NC}"
    echo -e "${CYAN}Summary:${NC}"
    echo ""

    for i in $(seq 0 $((TASK_COUNT - 1))); do
        local id="${TASK_ORDER[$i]}"
        local name="${TASKS[${id}_name]}"
        local installed="${TASKS[${id}_installed]}"
        local details="${TASKS[${id}_details]}"

        local detail_text=""
        [ -n "$details" ] && detail_text=" - $details"

        if [ "$installed" = "true" ]; then
            echo -e "${GREEN}[Installed] ${name}${detail_text}${NC}"
        else
            echo -e "${YELLOW}[Missing] ${name}${detail_text}${NC}"
        fi
    done

    echo ""
    echo -e "${MAGENTA}All done! Enjoy your new dev box.${NC}"
    echo -e "${GRAY}Tip: Log out and log back in to ensure all PATH changes take effect.${NC}"
    echo ""

    local url="https://wuu73.org/blog"
    echo -e "${CYAN}For useful links and information about cheap/free AI coding methods:${NC}"
    echo -e "${CYAN}${url}${NC}"
    echo ""

    read -r -p "Open the blog in your browser now? (Y/n): " response
    if [[ ! "$response" =~ ^[Nn] ]]; then
        if user_has_command xdg-open; then
            run_as_user "xdg-open '$url' >/dev/null 2>&1 &" || true
            write_ok "Opening browser..."
        elif user_has_command firefox; then
            run_as_user "firefox '$url' >/dev/null 2>&1 &" || true
            write_ok "Opening Firefox..."
        elif user_has_command google-chrome; then
            run_as_user "google-chrome '$url' >/dev/null 2>&1 &" || true
            write_ok "Opening Chrome..."
        else
            write_warn "No browser command found. Please visit: $url"
        fi
    fi

    echo ""
    write_quick_start_file
    echo ""
    echo -e "${GREEN}Installation complete!${NC}"
}

main "$@"
