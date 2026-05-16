#!/bin/sh
# Zebflow installer
# Usage: curl -fsSL https://raw.githubusercontent.com/zebflow/zebflow/main/install.sh | sh
#
# Environment variables:
#   ZEBFLOW_VERSION      - Version to install (default: latest)
#   ZEBFLOW_INSTALL_DIR  - Installation directory (default: /usr/local/bin or ~/.zebflow/bin)
#   ZEBFLOW_NO_VERIFY    - Skip checksum verification if set to 1
#   ZEBFLOW_DRY_RUN      - Download but don't install if set to 1

set -e

REPO="zebflow/zebflow"
GITHUB_API="https://api.github.com/repos/${REPO}/releases/latest"
GITHUB_DOWNLOAD="https://github.com/${REPO}/releases/download"

# --- Colors (if terminal supports it) ---

setup_colors() {
  if [ -t 1 ] && command -v tput >/dev/null 2>&1 && [ "$(tput colors 2>/dev/null || echo 0)" -ge 8 ]; then
    BOLD="$(tput bold)"
    GREEN="$(tput setaf 2)"
    YELLOW="$(tput setaf 3)"
    RED="$(tput setaf 1)"
    RESET="$(tput sgr0)"
  else
    BOLD="" GREEN="" YELLOW="" RED="" RESET=""
  fi
}

info()  { printf '%s[info]%s %s\n' "$GREEN" "$RESET" "$1"; }
warn()  { printf '%s[warn]%s %s\n' "$YELLOW" "$RESET" "$1"; }
error() { printf '%s[error]%s %s\n' "$RED" "$RESET" "$1" >&2; }
fatal() { error "$1"; exit 1; }

# --- Platform detection ---

detect_platform() {
  OS="$(uname -s)"
  ARCH="$(uname -m)"

  case "$OS" in
    Linux)   PLATFORM="linux" ;;
    Darwin)  PLATFORM="darwin" ;;
    MINGW*|MSYS*|CYGWIN*)  PLATFORM="windows" ;;
    *)       fatal "Unsupported operating system: $OS" ;;
  esac

  case "$ARCH" in
    x86_64|amd64)   ARCH="amd64" ;;
    aarch64|arm64)   ARCH="arm64" ;;
    *)               fatal "Unsupported architecture: $ARCH" ;;
  esac

  # Validate supported combinations
  case "${PLATFORM}-${ARCH}" in
    linux-amd64|linux-arm64|darwin-arm64|windows-amd64) ;;
    darwin-amd64) fatal "macOS Intel (x86_64) is not supported. Zebflow requires Apple Silicon (arm64)." ;;
    *) fatal "Unsupported platform: ${PLATFORM}-${ARCH}" ;;
  esac

  if [ "$PLATFORM" = "windows" ]; then
    ARCHIVE="zebflow-${PLATFORM}-${ARCH}.zip"
    BINARY="zebflow.exe"
  else
    ARCHIVE="zebflow-${PLATFORM}-${ARCH}.tar.gz"
    BINARY="zebflow"
  fi
}

# --- Version resolution ---

resolve_version() {
  if [ -n "${ZEBFLOW_VERSION:-}" ]; then
    VERSION="$ZEBFLOW_VERSION"
    info "Using specified version: $VERSION"
    return
  fi

  info "Fetching latest version..."

  if command -v curl >/dev/null 2>&1; then
    RESPONSE=$(curl -fsSL "$GITHUB_API" 2>/dev/null) || fatal "Failed to fetch latest version from GitHub API"
  elif command -v wget >/dev/null 2>&1; then
    RESPONSE=$(wget -qO- "$GITHUB_API" 2>/dev/null) || fatal "Failed to fetch latest version from GitHub API"
  else
    fatal "Neither curl nor wget found. Please install one of them."
  fi

  # Parse tag_name from JSON without jq
  VERSION=$(printf '%s' "$RESPONSE" | grep '"tag_name"' | head -1 | sed 's/.*"tag_name"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/')

  if [ -z "$VERSION" ]; then
    fatal "Could not determine latest version. Set ZEBFLOW_VERSION manually."
  fi

  info "Latest version: $VERSION"
}

# --- Download ---

download_file() {
  URL="$1"
  DEST="$2"

  if command -v curl >/dev/null 2>&1; then
    curl -fsSL -o "$DEST" "$URL" || return 1
  elif command -v wget >/dev/null 2>&1; then
    wget -qO "$DEST" "$URL" || return 1
  fi
}

download() {
  TMPDIR=$(mktemp -d)
  trap 'rm -rf "$TMPDIR"' EXIT

  DOWNLOAD_URL="${GITHUB_DOWNLOAD}/${VERSION}/${ARCHIVE}"
  info "Downloading ${ARCHIVE}..."

  download_file "$DOWNLOAD_URL" "${TMPDIR}/${ARCHIVE}" || \
    fatal "Download failed: ${DOWNLOAD_URL}\nCheck that version ${VERSION} exists and has release assets."

  # Checksum verification
  if [ "${ZEBFLOW_NO_VERIFY:-0}" != "1" ]; then
    CHECKSUMS_URL="${GITHUB_DOWNLOAD}/${VERSION}/zebflow-checksums.txt"
    if download_file "$CHECKSUMS_URL" "${TMPDIR}/checksums.txt" 2>/dev/null; then
      EXPECTED=$(grep "$ARCHIVE" "${TMPDIR}/checksums.txt" | awk '{print $1}')
      if [ -n "$EXPECTED" ]; then
        if command -v sha256sum >/dev/null 2>&1; then
          ACTUAL=$(sha256sum "${TMPDIR}/${ARCHIVE}" | awk '{print $1}')
        elif command -v shasum >/dev/null 2>&1; then
          ACTUAL=$(shasum -a 256 "${TMPDIR}/${ARCHIVE}" | awk '{print $1}')
        else
          warn "No sha256sum or shasum found — skipping checksum verification"
          ACTUAL="$EXPECTED"
        fi

        if [ "$EXPECTED" != "$ACTUAL" ]; then
          fatal "Checksum mismatch!\n  Expected: ${EXPECTED}\n  Got:      ${ACTUAL}\nThe download may be corrupted. Try again."
        fi
        info "Checksum verified"
      else
        warn "Checksum entry not found for ${ARCHIVE} — skipping verification"
      fi
    else
      warn "Checksums file not available — skipping verification"
    fi
  fi

  # Extract
  info "Extracting..."
  if [ "$PLATFORM" = "windows" ]; then
    unzip -qo "${TMPDIR}/${ARCHIVE}" -d "${TMPDIR}" || fatal "Extraction failed"
  else
    tar -xzf "${TMPDIR}/${ARCHIVE}" -C "${TMPDIR}" || fatal "Extraction failed"
  fi

  EXTRACTED="${TMPDIR}/${BINARY}"
  if [ ! -f "$EXTRACTED" ]; then
    fatal "Binary not found in archive. Expected: ${BINARY}"
  fi
}

# --- Install ---

determine_install_dir() {
  if [ -n "${ZEBFLOW_INSTALL_DIR:-}" ]; then
    INSTALL_DIR="$ZEBFLOW_INSTALL_DIR"
    return
  fi

  # Try /usr/local/bin first
  if [ -d "/usr/local/bin" ] && [ -w "/usr/local/bin" ]; then
    INSTALL_DIR="/usr/local/bin"
    return
  fi

  # Fallback to ~/.zebflow/bin
  INSTALL_DIR="${HOME}/.zebflow/bin"
}

install_binary() {
  determine_install_dir

  if [ "${ZEBFLOW_DRY_RUN:-0}" = "1" ]; then
    info "[dry-run] Would install to: ${INSTALL_DIR}/${BINARY}"
    info "[dry-run] Binary size: $(wc -c < "$EXTRACTED" | tr -d ' ') bytes"
    return
  fi

  mkdir -p "$INSTALL_DIR"
  cp "$EXTRACTED" "${INSTALL_DIR}/${BINARY}"
  chmod +x "${INSTALL_DIR}/${BINARY}"

  info "Installed to ${INSTALL_DIR}/${BINARY}"

  # Check if install dir is in PATH
  case ":$PATH:" in
    *":${INSTALL_DIR}:"*) ;;
    *)
      warn "${INSTALL_DIR} is not in your PATH"
      printf '\n'
      printf '  Add it to your shell profile:\n'
      printf '    export PATH="%s:$PATH"\n' "$INSTALL_DIR"
      printf '\n'
      ;;
  esac
}

# --- Main ---

main() {
  setup_colors

  printf '\n'
  printf '  %sZebflow Installer%s\n' "$BOLD" "$RESET"
  printf '\n'

  detect_platform
  info "Detected platform: ${PLATFORM}-${ARCH}"

  resolve_version
  download
  install_binary

  printf '\n'
  printf '  %s✓ zebflow %s installed successfully%s\n' "$GREEN" "$VERSION" "$RESET"
  printf '\n'
  printf '  Quick start:\n'
  printf '    zebflow              # start server on port 10610\n'
  printf '    zebflow --help       # show options\n'
  printf '\n'
  printf '  Docs: https://github.com/%s\n' "$REPO"
  printf '\n'
}

main
