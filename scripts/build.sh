#!/usr/bin/env bash
set -euo pipefail

# Build gcli for all platforms:
#   - macOS ARM64: built locally
#   - Linux x86_64: built on server1 via podman
#
# Usage: ./scripts/build.sh
#
# Prerequisites: commit and push changes before running.
# Outputs binaries to build/out/

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
SSH="ssh -o ServerAliveInterval=30"
REMOTE="core@server1.g10.lo"
REMOTE_SRC="/var/gcli/src"
REMOTE_CARGO_REGISTRY="/var/gcli/cargo-registry"
REMOTE_CARGO_GIT="/var/gcli/cargo-git"
REMOTE_TARGET="/var/gcli/target"
BUILDER_IMAGE="gcli-builder"
OUT_DIR="$PROJECT_DIR/build/out"

mkdir -p "$OUT_DIR"

echo "=== Building gcli ==="

# --- Clone or pull source on server1 ---
echo "--- Updating source on server1 ---"
$SSH "$REMOTE" "sudo mkdir -p /var/gcli && sudo chown -R \$(id -u):\$(id -g) /var/gcli && \
    mkdir -p $REMOTE_CARGO_REGISTRY $REMOTE_CARGO_GIT $REMOTE_TARGET && \
    if [ -d $REMOTE_SRC/.git ]; then \
        cd $REMOTE_SRC && git pull; \
    else \
        git clone https://github.com/glennswest/gcli.git $REMOTE_SRC; \
    fi"

echo "--- Building container image on server1 ---"
$SSH "$REMOTE" "cd $REMOTE_SRC && CONTAINER_HOST= podman build -t $BUILDER_IMAGE -f Containerfile ."

# --- Linux x86_64 build on server1 ---
echo "--- Building Linux x86_64 on server1 ---"
$SSH "$REMOTE" "CONTAINER_HOST= podman rm -f gcli-x86_64 2>/dev/null; \
    CONTAINER_HOST= podman run --rm --name gcli-x86_64 \
    --security-opt label=disable \
    -v $REMOTE_SRC:/src:ro \
    -v $REMOTE_CARGO_REGISTRY:/usr/local/cargo/registry \
    -v $REMOTE_CARGO_GIT:/usr/local/cargo/git \
    -v $REMOTE_TARGET:/build/target \
    $BUILDER_IMAGE bash -c 'cd /src && CARGO_TARGET_DIR=/build/target \
        cargo build --release && strip /build/target/release/gcli'"

echo "--- Fetching Linux x86_64 binary ---"
scp "$REMOTE:$REMOTE_TARGET/release/gcli" "$OUT_DIR/gcli-linux-x86_64"

# --- macOS ARM64 build locally ---
echo "--- Building macOS ARM64 locally ---"
(cd "$PROJECT_DIR" && cargo build --release)
cp "$PROJECT_DIR/target/release/gcli" "$OUT_DIR/gcli-macos-arm64"
strip "$OUT_DIR/gcli-macos-arm64"

echo ""
echo "=== Build complete ==="
ls -lh "$OUT_DIR"/gcli-*
