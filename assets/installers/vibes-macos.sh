#!/bin/bash
#
# vibes-macos.sh
# Quick bootstrap installer for AI coding tools on macOS.
#
# Run this as your normal user from Terminal or iTerm:
#   bash vibes-macos.sh
#

set -euo pipefail
IFS=$'\n\t'

if [ "${EUID}" -eq 0 ]; then
    echo "Please run this script as your normal macOS user, not as root."
    exit 1
fi

write_info() { echo "-> $1"; }
write_ok() { echo "OK: $1"; }
write_warn() { echo "!! $1"; }

DESKTOP_DIR="${HOME}/Desktop"
mkdir -p "$DESKTOP_DIR" 2>/dev/null || true
LOG_FILE="${DESKTOP_DIR}/vibes-macos-$(date +%Y%m%d-%H%M%S).log"
touch "$LOG_FILE"
exec > >(tee -a "$LOG_FILE") 2>&1

ensure_xcode_tools() {
    if xcode-select -p >/dev/null 2>&1; then
        return
    fi

    write_warn "Xcode Command Line Tools are required before Homebrew can finish installing."
    xcode-select --install || true
    write_warn "Please complete the Command Line Tools install dialog, then run this script again."
    exit 1
}

ensure_homebrew() {
    if command -v brew >/dev/null 2>&1; then
        return
    fi

    write_info "Installing Homebrew..."
    NONINTERACTIVE=1 /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)"
}

load_brew_env() {
    if [ -x "/opt/homebrew/bin/brew" ]; then
        eval "$(/opt/homebrew/bin/brew shellenv)"
    elif [ -x "/usr/local/bin/brew" ]; then
        eval "$(/usr/local/bin/brew shellenv)"
    fi
}

brew_install_formula() {
    local package="$1"
    if brew list --formula "$package" >/dev/null 2>&1; then
        write_ok "$package already present"
        return
    fi

    write_info "Installing $package..."
    brew install "$package"
}

brew_install_cask() {
    local package="$1"
    if brew list --cask "$package" >/dev/null 2>&1; then
        write_ok "$package already present"
        return
    fi

    write_info "Installing $package..."
    brew install --cask "$package"
}

ensure_python_stack() {
    brew_install_formula python

    python3 -m pip install --user --upgrade pip >/dev/null
    python3 -m pip install --user --upgrade pipx >/dev/null
    python3 -m pipx ensurepath >/dev/null

    if command -v uv >/dev/null 2>&1; then
        write_ok "uv already present"
    else
        python3 -m pip install --user --upgrade uv >/dev/null
        write_ok "uv installed"
    fi
}

ensure_user_path() {
    local extra_paths=(
        "${HOME}/.local/bin"
        "${HOME}/Library/Application Support/pipx/bin"
        "/Applications/Visual Studio Code.app/Contents/Resources/app/bin"
    )

    for python_bin in "${HOME}"/Library/Python/*/bin; do
        if [ -d "$python_bin" ]; then
            extra_paths+=("$python_bin")
        fi
    done

    for extra_path in "${extra_paths[@]}"; do
        if [ -d "$extra_path" ] && [[ ":${PATH}:" != *":${extra_path}:"* ]]; then
            PATH="${extra_path}:${PATH}"
        fi
    done

    export PATH
}

install_aicodeprep() {
    ensure_python_stack
    ensure_user_path

    if command -v aicodeprep-gui >/dev/null 2>&1; then
        write_ok "aicodeprep-gui already present"
        return
    fi

    write_info "Installing aicodeprep-gui..."
    python3 -m pipx install aicodeprep-gui >/dev/null || python3 -m pipx upgrade aicodeprep-gui >/dev/null || true
}

install_npm_global() {
    local package="$1"
    local command_name="$2"

    if command -v "$command_name" >/dev/null 2>&1; then
        write_ok "$package already present"
        return
    fi

    write_info "Installing $package..."
    npm install -g "$package"
    write_ok "$package installed"
}

install_vscode_extensions() {
    ensure_user_path

    if ! command -v code >/dev/null 2>&1; then
        write_warn "VS Code command line launcher is not available yet; skipping extensions for now."
        return
    fi

    local extensions=(
        "saoudrizwan.claude-dev"
        "kilocode.Kilo-Code"
        "ms-vscode.live-server"
        "oderwat.indent-rainbow"
        "esbenp.prettier-vscode"
    )

    for extension in "${extensions[@]}"; do
        write_info "Installing VS Code extension: ${extension}"
        code --install-extension "$extension" --force >/dev/null || true
    done
}

write_quick_start_file() {
    local quick_start_file="${DESKTOP_DIR}/Vibe-Coding-Quick-Start.txt"

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
   Open Visual Studio Code from Applications or Spotlight.
   Open Cline or Kilo Code from the left sidebar.
   Choose Qwen / Qwen Code as the provider if you want a free starting point.

Other useful commands:
   aicp
   codex
   claude
EOF

    write_ok "Created desktop quick-start guide: $quick_start_file"
}

main() {
    write_info "Preparing macOS AI coding bootstrap..."
    ensure_xcode_tools
    ensure_homebrew
    load_brew_env

    brew_install_formula git
    brew_install_formula node
    ensure_python_stack
    ensure_user_path
    install_aicodeprep

    brew_install_cask visual-studio-code
    ensure_user_path
    install_vscode_extensions

    install_npm_global "@qwen-code/qwen-code" "qwen"
    install_npm_global "@google/gemini-cli" "gemini"
    install_npm_global "@openai/codex" "codex"
    install_npm_global "@anthropic-ai/claude-code" "claude"

    write_quick_start_file

    echo ""
    write_ok "All done! Enjoy your new dev box."
    write_ok "Installer log saved to: $LOG_FILE"
}

main "$@"
