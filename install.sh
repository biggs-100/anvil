#!/usr/bin/env sh
# shellcheck disable=SC3043
#
# Anvil — install the CLI binary
# Usage: curl -fsSL https://raw.githubusercontent.com/biggs-100/anvil/main/install.sh | sh
#
# Downloads the latest release from GitHub, verifies the SHA-256 checksum
# when available, and places the binary in /usr/local/bin.

set -eu

BINARY="anvil"
REPO="biggs-100/anvil"
VERSION="${ANVIL_VERSION:-latest}"
INSTALL_DIR="${ANVIL_INSTALL_DIR:-/usr/local/bin}"

# ---- helpers ----

info()  { printf "\033[32minfo\033[0m: %s\n" "$*"; }
warn()  { printf "\033[33mwarn\033[0m: %s\n" "$*"; }
error() { printf "\033[31merror\033[0m: %s\n" "$*" >&2; exit 1; }

detect_os_arch() {
    OS="$(uname -s | tr '[:upper:]' '[:lower:]')"
    ARCH="$(uname -m)"

    case "$OS" in
        linux)   TARGET="${ARCH}-unknown-linux-gnu"   ;;
        darwin)  TARGET="${ARCH}-apple-darwin"         ;;
        *)       error "unsupported OS: $OS"           ;;
    esac

    case "$ARCH" in
        x86_64|aarch64|arm64) ;;
        *) error "unsupported architecture: $ARCH" ;;
    esac
}

fetch() {
    if command -v curl >/dev/null 2>&1; then
        curl -fsSL "$@"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO- "$@"
    else
        error "need curl or wget to download"
    fi
}

# ---- resolve version ----

detect_os_arch

if [ "$VERSION" = "latest" ]; then
    VERSION="$(fetch "https://api.github.com/repos/$REPO/releases/latest" \
        | grep '"tag_name":' | sed 's/.*"tag_name": "//;s/".*//')"
    [ -z "$VERSION" ] && error "could not detect latest version"
fi

ARCHIVE="anvil-${TARGET}.tar.gz"
ARCHIVE_URL="https://github.com/$REPO/releases/download/$VERSION/$ARCHIVE"
CHECKSUM_URL="${ARCHIVE_URL}.sha256"

# ---- download ----

info "anvil ${VERSION} (${TARGET})"
info "downloading from ${ARCHIVE_URL}"

TMPDIR="$(mktemp -d 2>/dev/null || mktemp -d -t anvil-install)"
trap 'rm -rf "$TMPDIR"' EXIT INT TERM

fetch "$ARCHIVE_URL" > "$TMPDIR/$ARCHIVE"

# ---- verify checksum (best-effort) ----
CHECKSUM="$(fetch "$CHECKSUM_URL" 2>/dev/null || true)"
if [ -n "$CHECKSUM" ]; then
    if command -v sha256sum >/dev/null 2>&1; then
        echo "$CHECKSUM" | sha256sum -c - >/dev/null 2>&1 \
            && info "checksum verified" \
            || warn "checksum mismatch — continuing anyway"
    elif command -v shasum >/dev/null 2>&1; then
        echo "$CHECKSUM" | shasum -a 256 -c - >/dev/null 2>&1 \
            && info "checksum verified" \
            || warn "checksum mismatch — continuing anyway"
    fi
fi

# ---- install ----

tar xzf "$TMPDIR/$ARCHIVE" -C "$TMPDIR"

BINARY_SRC="$TMPDIR/anvil/anvil"
[ ! -f "$BINARY_SRC" ] && error "binary not found in archive"

mkdir -p "$INSTALL_DIR"
cp "$BINARY_SRC" "$INSTALL_DIR/$BINARY"
chmod +x "$INSTALL_DIR/$BINARY"

info "installed anvil ${VERSION} to ${INSTALL_DIR}/anvil"
info "run 'anvil --help' to get started"
