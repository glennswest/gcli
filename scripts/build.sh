#!/usr/bin/env bash
set -euo pipefail

# Build gcli for all platforms:
#   - macOS ARM64: built locally
#   - Linux x86_64: built on server1 via podman
#
# Usage: ./scripts/build.sh
#
# Outputs binaries to build/out/

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
REMOTE="core@server1.g10.lo"
REMOTE_SRC="/var/gcli/src"
REMOTE_CARGO_REGISTRY="/var/gcli/cargo-registry"
REMOTE_CARGO_GIT="/var/gcli/cargo-git"
REMOTE_TARGET="/var/gcli/target"
BUILDER_IMAGE="gcli-builder"
OUT_DIR="$PROJECT_DIR/build/out"

mkdir -p "$OUT_DIR"

echo "=== Building gcli ==="

# --- Ensure builder image exists on server1 ---
echo "--- Syncing source to server1 ---"
ssh "$REMOTE" "sudo mkdir -p $REMOTE_SRC $REMOTE_CARGO_REGISTRY $REMOTE_CARGO_GIT $REMOTE_TARGET"
rsync -az --delete \
    --exclude target/ --exclude build/out/ --exclude .git/ \
    "$PROJECT_DIR/" "$REMOTE:$REMOTE_SRC/"

echo "--- Building container image on server1 ---"
ssh "$REMOTE" "cd $REMOTE_SRC && CONTAINER_HOST= podman build -t $BUILDER_IMAGE -f Containerfile ."

# --- Linux x86_64 build on server1 ---
echo "--- Building Linux x86_64 on server1 ---"
ssh "$REMOTE" "CONTAINER_HOST= podman run --rm --name gcli-x86_64 \
    --security-opt label=disable \
    -v $REMOTE_SRC:/src:ro \
    -v $REMOTE_CARGO_REGISTRY:/usr/local/cargo/registry \
    -v $REMOTE_CARGO_GIT:/usr/local/cargo/git \
    -v $REMOTE_TARGET:/build/target \
    $BUILDER_IMAGE bash -c 'cd /src && CARGO_TARGET_DIR=/build/target \
        cargo build --release && strip /build/target/release/gcli'"

echo "--- Fetching Linux x86_64 binary ---"
rsync -az "$REMOTE:$REMOTE_TARGET/release/gcli" "$OUT_DIR/gcli-linux-x86_64"

# --- macOS ARM64 build locally ---
echo "--- Building macOS ARM64 locally ---"
(cd "$PROJECT_DIR" && cargo build --release)
cp "$PROJECT_DIR/target/release/gcli" "$OUT_DIR/gcli-macos-arm64"
strip "$OUT_DIR/gcli-macos-arm64"

echo ""
echo "=== Build complete ==="
ls -lh "$OUT_DIR"/gcli-*
