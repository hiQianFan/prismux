#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TARGET_DIR="${CARGO_TARGET_DIR:-"$ROOT/target"}"
VERSION="$(awk '
  $0 == "[workspace.package]" { in_workspace = 1; next }
  /^\[/ && $0 != "[workspace.package]" { in_workspace = 0 }
  in_workspace && /^version = / {
    gsub(/"/, "", $3)
    print $3
    exit
  }
' "$ROOT/Cargo.toml")"

if [[ -z "$VERSION" ]]; then
  echo "error: could not read workspace package version from Cargo.toml" >&2
  exit 1
fi

TAG="${RELEASE_TAG:-"v$VERSION"}"
SUFFIX="${1:-macos-arm64}"
DIST_DIR="$ROOT/dist"
PACKAGE_NAME="prismux-cli-${TAG}-${SUFFIX}"
PACKAGE_DIR="$DIST_DIR/$PACKAGE_NAME"

cargo build --release -p prismux-cli --locked

rm -rf "$PACKAGE_DIR"
mkdir -p "$PACKAGE_DIR/bin"

cp "$TARGET_DIR/release/prismux" "$PACKAGE_DIR/bin/prismux"
cp "$TARGET_DIR/release/pmx" "$PACKAGE_DIR/bin/pmx"
chmod 0755 "$PACKAGE_DIR/bin/prismux" "$PACKAGE_DIR/bin/pmx"

cat > "$PACKAGE_DIR/install.sh" <<'SH'
#!/usr/bin/env bash
set -euo pipefail

SOURCE_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
INSTALL_DIR="${PRISMUX_INSTALL_DIR:-"$HOME/.local/bin"}"

mkdir -p "$INSTALL_DIR"
cp "$SOURCE_DIR/bin/prismux" "$INSTALL_DIR/prismux"
cp "$SOURCE_DIR/bin/pmx" "$INSTALL_DIR/pmx"
chmod 0755 "$INSTALL_DIR/prismux" "$INSTALL_DIR/pmx"

echo "Installed prismux CLI to $INSTALL_DIR"
echo
echo "Verify:"
echo "  prismux --version"
echo "  prismux status"
echo
case ":$PATH:" in
  *":$INSTALL_DIR:"*) ;;
  *)
    echo "Note: $INSTALL_DIR is not currently on PATH."
    echo "For zsh, add it with:"
    echo "  echo 'export PATH=\"\$HOME/.local/bin:\$PATH\"' >> ~/.zshrc"
    ;;
esac
SH
chmod 0755 "$PACKAGE_DIR/install.sh"

cat > "$PACKAGE_DIR/README.md" <<EOF
# Prismux CLI

This package contains the standalone Prismux CLI commands:

- \`prismux\`
- \`pmx\`

## Install

\`\`\`sh
./install.sh
\`\`\`

By default, this installs both commands to:

\`\`\`text
\$HOME/.local/bin
\`\`\`

To choose another install directory:

\`\`\`sh
PRISMUX_INSTALL_DIR=/usr/local/bin ./install.sh
\`\`\`

## Verify

\`\`\`sh
prismux --version
prismux status
\`\`\`

## First Commands

\`\`\`sh
prismux login codex
prismux login claude --alias work
prismux list
prismux use codex 2
\`\`\`

Prismux does not modify shell startup files automatically. If the install
directory is not on \`PATH\`, add it in your shell configuration.
EOF

mkdir -p "$DIST_DIR"
tar -C "$DIST_DIR" -czf "$DIST_DIR/$PACKAGE_NAME.tar.gz" "$PACKAGE_NAME"
rm -rf "$PACKAGE_DIR"

echo "$DIST_DIR/$PACKAGE_NAME.tar.gz"
