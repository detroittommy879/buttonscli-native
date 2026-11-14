#!/bin/sh
# ButtonsCLI installer for Linux and macOS.
#
#   curl -fsSL https://buttonscli.com/install.sh | sh
#
# Options (arguments go after `sh -s --`, e.g. `| sh -s -- --deps`):
#   --version X.Y.Z     install that release instead of the newest
#   --deps              Linux: install missing runtime libraries with sudo
#   --no-deps           Linux: never offer to install libraries
#   --no-modify-path    do not add the install folder to your shell's PATH
#   --uninstall         remove ButtonsCLI (keeps your settings)
# Environment: BUTTONSCLI_VERSION, BUTTONSCLI_INSTALL_DIR (default ~/.local/bin),
#   BUTTONSCLI_NO_MODIFY_PATH=1, BUTTONSCLI_DEPS=1|0.
#
# Everything runs as your user. sudo is only used for --deps (or when you
# answer yes to the library prompt). Settings live in ~/.buttonscli-native.

set -u

REPO="detroittommy879/buttonscli-native"
APP="ButtonsCLI"
MARK="# added by the ButtonsCLI installer"

say() { printf '%s\n' "$*"; }
info() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
warn() { printf '\033[1;33mwarning:\033[0m %s\n' "$*" >&2; }
die() {
    printf '\033[1;31merror:\033[0m %s\n' "$*" >&2
    exit 1
}
has() { command -v "$1" >/dev/null 2>&1; }

usage() {
    cat <<'EOF'
ButtonsCLI installer: curl -fsSL https://buttonscli.com/install.sh | sh -s -- [options]
  --version X.Y.Z   install that release instead of the newest
  --deps            Linux: install missing runtime libraries with sudo
  --no-deps         Linux: never offer to install libraries
  --no-modify-path  do not add the install folder to your shell's PATH
  --uninstall       remove ButtonsCLI (keeps your settings)
Environment: BUTTONSCLI_VERSION, BUTTONSCLI_INSTALL_DIR (default ~/.local/bin),
  BUTTONSCLI_NO_MODIFY_PATH=1, BUTTONSCLI_DEPS=1|0
EOF
}

# --- downloads ---------------------------------------------------------------

fetch() { # url dest
    if has curl; then
        curl --proto '=https' --tlsv1.2 -fsSL --retry 3 -o "$2" "$1"
    elif has wget; then
        wget -q --https-only -O "$2" "$1"
    else
        die "curl or wget is needed to download ButtonsCLI."
    fi
}

latest_version() {
    tag=""
    if has curl; then
        # github.com/.../releases/latest redirects to .../tag/vX.Y.Z: no API
        # call, so no API rate limit.
        url=$(curl --proto '=https' -fsSLI -o /dev/null -w '%{url_effective}' \
            "https://github.com/$REPO/releases/latest" 2>/dev/null) || url=""
        tag=${url##*/}
    fi
    case "$tag" in
    v[0-9]*) ;;
    *)
        json="$TMP/latest.json"
        fetch "https://api.github.com/repos/$REPO/releases/latest" "$json" ||
            die "Could not find the newest release. Set BUTTONSCLI_VERSION=X.Y.Z and try again."
        tag=$(sed -n 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p' "$json" | head -n 1)
        ;;
    esac
    case "$tag" in
    v[0-9]*) printf '%s\n' "${tag#v}" ;;
    *) die "Could not read the newest release tag (got '$tag')." ;;
    esac
}

sha256_of() {
    if has sha256sum; then
        sha256sum "$1" | cut -d ' ' -f 1
    elif has shasum; then
        shasum -a 256 "$1" | cut -d ' ' -f 1
    elif has openssl; then
        openssl dgst -sha256 "$1" | sed 's/.*= *//'
    else
        die "sha256sum, shasum or openssl is needed to check the download."
    fi
}

verify() { # file asset-name sums-file
    want=$(awk -v f="$2" '$2 == f || $2 == "*" f { print $1; exit }' "$3")
    [ -n "$want" ] || die "$2 is not listed in SHA256SUMS.txt."
    got=$(sha256_of "$1")
    [ "$(printf '%s' "$got" | tr 'A-F' 'a-f')" = "$(printf '%s' "$want" | tr 'A-F' 'a-f')" ] ||
        die "Checksum mismatch for $2 (expected $want, got $got). Nothing was installed."
}

download_release() { # asset -> prints local path
    base="https://github.com/$REPO/releases/download/v$VERSION"
    info "Downloading $1"
    fetch "$base/$1" "$TMP/$1" || die "Download failed: $base/$1"
    fetch "$base/SHA256SUMS.txt" "$TMP/SHA256SUMS.txt" || die "Download failed: $base/SHA256SUMS.txt"
    verify "$TMP/$1" "$1" "$TMP/SHA256SUMS.txt"
    info "Checksum OK"
}

# --- PATH --------------------------------------------------------------------

on_path() {
    case ":${PATH:-}:" in
    *":$1:"*) return 0 ;;
    esac
    return 1
}

rc_files() {
    case "${SHELL:-}" in
    */zsh) printf '%s\n' "${ZDOTDIR:-$HOME}/.zshrc" ;;
    */bash)
        printf '%s\n' "$HOME/.bashrc"
        # Login shells on macOS read .bash_profile, not .bashrc.
        [ "$OS" = macos ] && printf '%s\n' "$HOME/.bash_profile"
        ;;
    */fish) printf '%s\n' "${XDG_CONFIG_HOME:-$HOME/.config}/fish/conf.d/buttonscli.fish" ;;
    *) printf '%s\n' "$HOME/.profile" ;;
    esac
}

add_to_path() {
    if on_path "$BIN"; then
        return
    fi
    if [ "$MODIFY_PATH" = 0 ]; then
        warn "$BIN is not on your PATH. Add it yourself to run buttonscli by name."
        return
    fi
    for rc in $(rc_files); do
        if [ -f "$rc" ] && grep -qF "$MARK" "$rc"; then
            continue
        fi
        mkdir -p "$(dirname "$rc")"
        case "$rc" in
        *.fish) line="fish_add_path -g \"$BIN\" $MARK" ;;
        *) line="export PATH=\"$BIN:\$PATH\" $MARK" ;;
        esac
        printf '\n%s\n' "$line" >>"$rc"
        info "Added $BIN to PATH in $rc (open a new terminal to use it)"
    done
}

remove_from_path() {
    for rc in "$HOME/.bashrc" "$HOME/.bash_profile" "$HOME/.profile" \
        "${ZDOTDIR:-$HOME}/.zshrc"; do
        if [ -f "$rc" ] && grep -qF "$MARK" "$rc"; then
            { grep -vF "$MARK" "$rc" || true; } >"$rc.buttonscli-tmp" && cat "$rc.buttonscli-tmp" >"$rc"
            rm -f "$rc.buttonscli-tmp"
            info "Removed the PATH line from $rc"
        fi
    done
    rm -f "${XDG_CONFIG_HOME:-$HOME/.config}/fish/conf.d/buttonscli.fish"
}

# --- Linux -------------------------------------------------------------------

DATA="${XDG_DATA_HOME:-$HOME/.local/share}"
DESKTOP="$DATA/applications/buttonscli.desktop"
ICON="$DATA/icons/hicolor/256x256/apps/buttonscli.png"

distro_family() {
    ids=""
    if [ -r /etc/os-release ]; then
        ids=$(. /etc/os-release && printf '%s %s' "${ID:-}" "${ID_LIKE:-}")
    fi
    for id in $ids; do
        case "$id" in
        debian | ubuntu) echo debian && return ;;
        fedora | rhel | centos) echo fedora && return ;;
        arch) echo arch && return ;;
        suse | opensuse* | sles) echo suse && return ;;
        alpine) echo alpine && return ;;
        esac
    done
    echo unknown
}

# Runtime libraries. winit and wgpu open most of these at run time, so `ldd`
# cannot see them: look them up by soname instead.
have_lib() {
    if has ldconfig && ldconfig -p 2>/dev/null | grep -q "$1"; then
        return 0
    fi
    if [ -x /sbin/ldconfig ] && /sbin/ldconfig -p 2>/dev/null | grep -q "$1"; then
        return 0
    fi
    for dir in /lib /lib64 /usr/lib /usr/lib64 /usr/lib/x86_64-linux-gnu /lib/x86_64-linux-gnu \
        /usr/lib/aarch64-linux-gnu /usr/local/lib; do
        [ -e "$dir/$1" ] && return 0
    done
    return 1
}

missing_libs() {
    out=""
    have_lib libxkbcommon.so.0 || out="$out libxkbcommon"
    if ! have_lib libwayland-client.so.0 && ! have_lib libX11.so.6; then
        out="$out wayland-or-x11"
    fi
    have_lib libX11.so.6 && ! have_lib libxkbcommon-x11.so.0 && out="$out libxkbcommon-x11"
    if have_lib libX11.so.6; then
        have_lib libXcursor.so.1 || out="$out libXcursor"
        have_lib libXi.so.6 || out="$out libXi"
        have_lib libXrandr.so.2 || out="$out libXrandr"
    fi
    if ! have_lib libvulkan.so.1 && ! have_lib libEGL.so.1; then
        out="$out vulkan-or-egl"
    fi
    printf '%s' "$out"
}

deps_command() {
    case "$(distro_family)" in
    debian) echo "apt-get install -y libxkbcommon0 libxkbcommon-x11-0 libwayland-client0 libx11-6 libxcursor1 libxi6 libxrandr2 libvulkan1 mesa-vulkan-drivers libegl1" ;;
    fedora) echo "dnf install -y libxkbcommon libxkbcommon-x11 libwayland-client libX11 libXcursor libXi libXrandr vulkan-loader mesa-vulkan-drivers mesa-libEGL" ;;
    arch) echo "pacman -S --needed --noconfirm libxkbcommon libxkbcommon-x11 wayland libx11 libxcursor libxi libxrandr vulkan-icd-loader mesa" ;;
    suse) echo "zypper --non-interactive install libxkbcommon0 libxkbcommon-x11-0 libwayland-client0 libX11-6 libXcursor1 libXi6 libXrandr2 libvulkan1 Mesa-libEGL1 Mesa-vulkan-device-select" ;;
    *) echo "" ;;
    esac
}

ask_yes() { # question -> 0 for yes
    # /dev/tty can exist without a controlling terminal (CI, containers).
    (exec </dev/tty >/dev/tty) 2>/dev/null || return 1
    printf '%s [Y/n] ' "$1" >/dev/tty
    read -r answer </dev/tty || return 1
    case "$answer" in
    "" | y | Y | yes | Yes) return 0 ;;
    esac
    return 1
}

check_deps() {
    missing=$(missing_libs)
    [ -n "$missing" ] || return 0
    cmd=$(deps_command)
    warn "Some libraries ButtonsCLI needs to open a window are missing:$missing"
    if [ -z "$cmd" ]; then
        say "  Install your distro's packages for: xkbcommon (+x11), Wayland or X11 client libs,"
        say "  libXcursor, libXi, libXrandr, and a Vulkan loader with Mesa drivers (or EGL)."
        return 0
    fi
    sudo=""
    [ "$(id -u)" = 0 ] || sudo="sudo "
    if [ "$DEPS" = 0 ]; then
        say "  Install them with: $sudo$cmd"
        return 0
    fi
    if [ "$DEPS" = 1 ] || ask_yes "Install them now with: $sudo$cmd ?"; then
        info "Installing libraries"
        # shellcheck disable=SC2086
        if [ -n "$sudo" ]; then sudo $cmd; else $cmd; fi || warn "The package manager failed; run it yourself: $sudo$cmd"
    else
        say "  Install them later with: $sudo$cmd"
    fi
}

check_glibc() {
    if ldd --version 2>&1 | grep -qi musl; then
        die "This Linux uses musl (e.g. Alpine). ButtonsCLI builds need glibc 2.35 or newer."
    fi
    v=$(getconf GNU_LIBC_VERSION 2>/dev/null | awk '{print $2}')
    [ -n "$v" ] || return 0
    major=${v%%.*}
    minor=${v#*.}
    minor=${minor%%.*}
    if [ "$major" -lt 2 ] || { [ "$major" -eq 2 ] && [ "$minor" -lt 35 ]; }; then
        warn "glibc $v is older than 2.35 (Ubuntu 22.04 / Debian 12 / Fedora 36). ButtonsCLI may not start."
    fi
}

put_file() { # src dest mode: replace via rename, safe while the app runs
    mkdir -p "$(dirname "$2")"
    cp "$1" "$2.new" && chmod "$3" "$2.new" && mv -f "$2.new" "$2" || die "Could not write $2"
}

install_linux() {
    [ "$ARCH" = x86_64 ] || die "There is no Linux $ARCH build yet (only x86_64). Build from source: cargo install --git https://github.com/$REPO --locked buttonscli"
    check_glibc
    asset="buttonscli-$VERSION-linux-x86_64.tar.gz"
    download_release "$asset"
    tar -xzf "$TMP/$asset" -C "$TMP" || die "Could not unpack $asset"
    src="$TMP/buttonscli-$VERSION-linux-x86_64"
    [ -f "$src/buttonscli" ] || die "The archive has no buttonscli binary."
    put_file "$src/buttonscli" "$BIN/buttonscli" 755
    [ -f "$src/buttonsclictl" ] && put_file "$src/buttonsclictl" "$BIN/buttonsclictl" 755
    info "Installed $BIN/buttonscli"

    # Launcher entry and icon (newer archives carry them; older ones don't).
    if [ -f "$src/buttonscli.png" ]; then
        put_file "$src/buttonscli.png" "$ICON" 644
    elif fetch "https://raw.githubusercontent.com/$REPO/v$VERSION/packaging/icon-256.png" "$TMP/icon.png"; then
        put_file "$TMP/icon.png" "$ICON" 644
    else
        warn "Could not download the icon; the launcher entry will use a generic one."
    fi
    mkdir -p "$(dirname "$DESKTOP")"
    cat >"$DESKTOP.new" <<EOF
[Desktop Entry]
Type=Application
Name=ButtonsCLI
Comment=Terminal workspace with command buttons, tiled panes and themes
Exec="$BIN/buttonscli"
Icon=$ICON
Terminal=false
Categories=System;TerminalEmulator;Development;
Keywords=terminal;shell;ssh;console;
StartupWMClass=buttonscli
EOF
    mv -f "$DESKTOP.new" "$DESKTOP"
    has update-desktop-database && update-desktop-database "$(dirname "$DESKTOP")" >/dev/null 2>&1
    has gtk-update-icon-cache && gtk-update-icon-cache -q -t "$DATA/icons/hicolor" >/dev/null 2>&1
    info "Added ButtonsCLI to your app launcher"

    check_deps
    if grep -qi microsoft /proc/version 2>/dev/null; then
        say ""
        say "WSL detected: ButtonsCLI opens through WSLg (Windows 11). For a native Windows"
        say "app instead, run in PowerShell: irm https://buttonscli.com/install.ps1 | iex"
    fi
}

uninstall_linux() {
    rm -f "$BIN/buttonscli" "$BIN/buttonsclictl" "$DESKTOP" "$ICON"
    has update-desktop-database && update-desktop-database "$(dirname "$DESKTOP")" >/dev/null 2>&1
}

# --- macOS -------------------------------------------------------------------

mac_app_dir() {
    if [ -w /Applications ]; then echo /Applications; else echo "$HOME/Applications"; fi
}

install_macos() {
    asset="$APP-$VERSION-macos-universal.dmg"
    download_release "$asset"
    mnt="$TMP/mnt"
    mkdir -p "$mnt"
    hdiutil attach -nobrowse -readonly -quiet -mountpoint "$mnt" "$TMP/$asset" ||
        die "Could not open $asset"
    apps=$(mac_app_dir)
    mkdir -p "$apps"
    rm -rf "$apps/$APP.app.new"
    cp -R "$mnt/$APP.app" "$apps/$APP.app.new"
    hdiutil detach -quiet "$mnt" || true
    rm -rf "$apps/$APP.app" && mv "$apps/$APP.app.new" "$apps/$APP.app" ||
        die "Could not write $apps/$APP.app (is ButtonsCLI running?)"
    # The build is ad-hoc signed, not notarized. curl downloads carry no
    # quarantine flag, but clear it in case one was added.
    xattr -dr com.apple.quarantine "$apps/$APP.app" 2>/dev/null || true
    mkdir -p "$BIN"
    ln -sf "$apps/$APP.app/Contents/MacOS/buttonscli" "$BIN/buttonscli"
    ln -sf "$apps/$APP.app/Contents/MacOS/buttonsclictl" "$BIN/buttonsclictl"
    info "Installed $apps/$APP.app (command: $BIN/buttonscli)"
}

uninstall_macos() {
    for apps in /Applications "$HOME/Applications"; do
        [ -d "$apps/$APP.app" ] && rm -rf "$apps/$APP.app" && info "Removed $apps/$APP.app"
    done
    rm -f "$BIN/buttonscli" "$BIN/buttonsclictl"
}

# --- main --------------------------------------------------------------------

main() {
    VERSION="${BUTTONSCLI_VERSION:-}"
    BIN="${BUTTONSCLI_INSTALL_DIR:-$HOME/.local/bin}"
    MODIFY_PATH=1
    [ "${BUTTONSCLI_NO_MODIFY_PATH:-0}" = 1 ] && MODIFY_PATH=0
    DEPS="${BUTTONSCLI_DEPS:-ask}"
    UNINSTALL=0
    while [ $# -gt 0 ]; do
        case "$1" in
        --version) [ $# -ge 2 ] || die "--version needs a value"; VERSION="$2"; shift ;;
        --version=*) VERSION="${1#*=}" ;;
        --deps) DEPS=1 ;;
        --no-deps) DEPS=0 ;;
        --no-modify-path) MODIFY_PATH=0 ;;
        --uninstall) UNINSTALL=1 ;;
        -h | --help) usage; exit 0 ;;
        *) die "Unknown option: $1 (try --help)" ;;
        esac
        shift
    done
    VERSION="${VERSION#v}"

    case "$(uname -s)" in
    Linux) OS=linux ;;
    Darwin) OS=macos ;;
    MINGW* | MSYS* | CYGWIN*) die "On Windows run in PowerShell: irm https://buttonscli.com/install.ps1 | iex" ;;
    *) die "Unsupported system: $(uname -s)" ;;
    esac
    case "$(uname -m)" in
    x86_64 | amd64) ARCH=x86_64 ;;
    aarch64 | arm64) ARCH=arm64 ;;
    *) ARCH=$(uname -m) ;;
    esac
    [ -n "${HOME:-}" ] || die "HOME is not set."

    if [ "$UNINSTALL" = 1 ]; then
        if [ "$OS" = linux ]; then uninstall_linux; else uninstall_macos; fi
        remove_from_path
        info "ButtonsCLI removed. Settings are kept in ~/.buttonscli-native (delete it to remove them too)."
        exit 0
    fi

    if [ "$(id -u)" = 0 ] && [ "${BUTTONSCLI_ALLOW_ROOT:-0}" != 1 ]; then
        die "Run this as your normal user, not root (it installs into your home folder). Set BUTTONSCLI_ALLOW_ROOT=1 to override."
    fi

    TMP=$(mktemp -d 2>/dev/null || mktemp -d -t buttonscli) || die "Could not create a temporary folder."
    trap 'rm -rf "$TMP"' EXIT INT TERM
    [ -n "$VERSION" ] || VERSION=$(latest_version)
    info "Installing ButtonsCLI $VERSION"

    if [ "$OS" = linux ]; then install_linux; else install_macos; fi
    add_to_path

    say ""
    info "Done. Start it from your app launcher, or run: buttonscli"
    on_path "$BIN" || say "    (in this terminal: $BIN/buttonscli)"
    say "    Uninstall: curl -fsSL https://buttonscli.com/install.sh | sh -s -- --uninstall"
}

main "$@"
