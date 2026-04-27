#!/usr/bin/env sh
# install.sh — Rune installer
# Usage:
#   curl -fsSL https://raw.githubusercontent.com/Akshay2642005/rune/master/install.sh | sh
#   curl -fsSL ... | sh -s -- --version v0.1.1-alpha
#   curl -fsSL ... | sh -s -- --bin runectl   # only CLI
#   curl -fsSL ... | sh -s -- --bin server    # only server
#   curl -fsSL ... | sh -s -- runectl         # positional (same as --bin runectl)
set -eu

REPO="Akshay2642005/rune"
INSTALL_DIR="${RUNE_INSTALL_DIR:-/usr/local/bin}"
BIN_FILTER="all"  # all | runectl | server
CLI_VERSION=""

while [ "$#" -gt 0 ]; do
    case "$1" in
        --bin)
            shift
            [ "$#" -gt 0 ] || { echo "error: --bin requires a value" >&2; exit 1; }
            BIN_FILTER="$1"
            ;;
        --version)
            shift
            [ "$#" -gt 0 ] || { echo "error: --version requires a value" >&2; exit 1; }
            CLI_VERSION="$1"
            ;;
        --help|-h)
            echo "usage: install.sh [--bin all|runectl|server] [--version <tag>]" >&2
            exit 0
            ;;
        --*)
            echo "error: unknown flag '$1'" >&2
            exit 1
            ;;
        *)
            if [ "$BIN_FILTER" = "all" ]; then
                BIN_FILTER="$1"
            else
                echo "error: unexpected argument '$1'" >&2
                exit 1
            fi
            ;;
    esac
    shift
done

# ── Detect version ────────────────────────────────────────────────────────────
if [ -n "$CLI_VERSION" ]; then
    VERSION="$CLI_VERSION"
elif [ -n "${RUNE_VERSION:-}" ]; then
    VERSION="$RUNE_VERSION"
else
    VERSION=$(curl -fsSL "https://api.github.com/repos/${REPO}/releases/latest" \
        | grep '"tag_name"' | sed 's/.*"tag_name": *"\([^"]*\)".*/\1/')
fi

if [ -z "$VERSION" ]; then
    echo "error: could not determine latest release version" >&2
    exit 1
fi

echo "Installing Rune ${VERSION}..."

# ── Detect platform ───────────────────────────────────────────────────────────
OS=$(uname -s)
ARCH=$(uname -m)

case "$OS" in
    Linux)
        case "$ARCH" in
            x86_64)  TARGET="x86_64-unknown-linux-gnu" ;;
            aarch64) TARGET="aarch64-unknown-linux-gnu" ;;
            arm64)   TARGET="aarch64-unknown-linux-gnu" ;;
            *)       echo "error: unsupported Linux arch: $ARCH" >&2; exit 1 ;;
        esac
        ;;
    Darwin)
        case "$ARCH" in
            x86_64) TARGET="x86_64-apple-darwin" ;;
            arm64)  TARGET="aarch64-apple-darwin" ;;
            *)      echo "error: unsupported macOS arch: $ARCH" >&2; exit 1 ;;
        esac
        ;;
    MINGW*|MSYS*|CYGWIN*)
        TARGET="x86_64-pc-windows-msvc"
        EXE=".exe"
        ;;
    *)
        echo "error: unsupported OS: $OS" >&2
        exit 1
        ;;
esac

EXE="${EXE:-}"
BASE_URL="https://github.com/${REPO}/releases/download/${VERSION}"

# ── Download + install helper ─────────────────────────────────────────────────
install_bin() {
    NAME="$1"         # archive name without extension, e.g. rune-server-x86_64-...
    BIN_NAME="$2"     # the actual binary inside the archive, e.g. rune-server

    ARCHIVE="${NAME}.tar.gz"
    URL="${BASE_URL}/${ARCHIVE}"

    echo "  → Downloading ${ARCHIVE}"
    TMP=$(mktemp -d)
    trap 'rm -rf "$TMP"' EXIT

    if command -v curl > /dev/null 2>&1; then
        curl -fsSL "$URL" -o "${TMP}/${ARCHIVE}"
    elif command -v wget > /dev/null 2>&1; then
        wget -q "$URL" -O "${TMP}/${ARCHIVE}"
    else
        echo "error: curl or wget required" >&2
        exit 1
    fi

    # Verify checksum (sha256sum preferred; fallback to shasum -a 256 if available)
    SHA_URL="${URL}.sha256"
    if command -v sha256sum > /dev/null 2>&1; then
        curl -fsSL "$SHA_URL" -o "${TMP}/${ARCHIVE}.sha256" 2>/dev/null || true
        if [ -f "${TMP}/${ARCHIVE}.sha256" ]; then
            (cd "$TMP" && sha256sum -c "${ARCHIVE}.sha256" --quiet)
            echo "  ✓ Checksum verified"
        fi
    elif command -v shasum > /dev/null 2>&1; then
        curl -fsSL "$SHA_URL" -o "${TMP}/${ARCHIVE}.sha256" 2>/dev/null || true
        if [ -f "${TMP}/${ARCHIVE}.sha256" ]; then
            (cd "$TMP" && shasum -a 256 -c "${ARCHIVE}.sha256" >/dev/null)
            echo "  ✓ Checksum verified"
        fi
    fi

    tar -xzf "${TMP}/${ARCHIVE}" -C "$TMP"

    if [ ! -f "${TMP}/${BIN_NAME}${EXE}" ]; then
        echo "error: binary '${BIN_NAME}${EXE}' not found in archive" >&2
        exit 1
    fi

    mkdir -p "$INSTALL_DIR"
    install -m755 "${TMP}/${BIN_NAME}${EXE}" "${INSTALL_DIR}/${BIN_NAME}${EXE}"
    echo "  ✓ Installed ${BIN_NAME} → ${INSTALL_DIR}/${BIN_NAME}${EXE}"
}

# ── Install selected binaries ─────────────────────────────────────────────────
case "$BIN_FILTER" in
    all)
        install_bin "rune-server-${TARGET}" "rune-server"
        install_bin "runectl-${TARGET}"     "rune"
        ;;
    server)
        install_bin "rune-server-${TARGET}" "rune-server"
        ;;
    runectl|cli)
        install_bin "runectl-${TARGET}"     "rune"
        ;;
    *)
        echo "error: unknown --bin value '${BIN_FILTER}' (use: all, server, runectl)" >&2
        exit 1
        ;;
esac

echo ""
echo "Done! Rune ${VERSION} is installed."
if [ "$BIN_FILTER" = "all" ] || [ "$BIN_FILTER" = "server" ]; then
    echo "  Run the server:  rune-server"
fi
if [ "$BIN_FILTER" = "all" ] || [ "$BIN_FILTER" = "runectl" ] || [ "$BIN_FILTER" = "cli" ]; then
    echo "  Deploy a function:  rune deploy --id hello --route /hello hello.wasm"
fi
echo ""
echo "Make sure ${INSTALL_DIR} is in your PATH."
