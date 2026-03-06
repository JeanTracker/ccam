#!/usr/bin/env bash
set -euo pipefail

REPO="JeanTracker/ccam"
BIN="ccam"
INSTALL_DIR="$HOME/.local/bin"

# Detect architecture
ARCH="$(uname -m)"
case "$ARCH" in
  arm64)  TARGET="aarch64-apple-darwin" ;;
  x86_64) TARGET="x86_64-apple-darwin" ;;
  *)
    echo "Unsupported architecture: $ARCH" >&2
    exit 1
    ;;
esac

# Detect OS
if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "ccam only supports macOS." >&2
  exit 1
fi

TARBALL="${BIN}-${TARGET}.tar.gz"
URL="https://github.com/${REPO}/releases/latest/download/${TARBALL}"

echo "Downloading ccam for ${TARGET}..."
TMP_DIR="$(mktemp -d)"
trap 'rm -rf "$TMP_DIR"' EXIT

curl -fsSL "$URL" -o "${TMP_DIR}/${TARBALL}"
tar -xzf "${TMP_DIR}/${TARBALL}" -C "$TMP_DIR"

mkdir -p "$INSTALL_DIR"
mv "${TMP_DIR}/${BIN}" "${INSTALL_DIR}/${BIN}"
chmod +x "${INSTALL_DIR}/${BIN}"

echo ""
echo "ccam $("${INSTALL_DIR}/${BIN}" --version) installed to ${INSTALL_DIR}/${BIN}"
echo ""
echo "Add the following to your shell config:"
echo ""
echo "  zsh  (~/.zshrc):"
echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
echo "    eval \"\$(ccam init zsh)\""
echo ""
echo "  bash (~/.bashrc):"
echo "    export PATH=\"\$HOME/.local/bin:\$PATH\""
echo "    eval \"\$(ccam init bash)\""
echo ""
echo "  fish (~/.config/fish/config.fish):"
echo "    fish_add_path \"\$HOME/.local/bin\""
echo "    ccam init fish | source"
echo ""
echo "Then restart your shell or run: source ~/.zshrc"
