#!/usr/bin/env bash
set -euo pipefail

# Usage: ./scripts/release.sh [major|minor|patch]
#
# Bumps version, builds all platforms, commits, tags, pushes,
# and creates a GitHub release with binaries attached.

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$PROJECT_DIR"

BUMP_TYPE="${1:-patch}"

if [[ "$BUMP_TYPE" != "major" && "$BUMP_TYPE" != "minor" && "$BUMP_TYPE" != "patch" ]]; then
    echo "Usage: $0 [major|minor|patch]"
    exit 1
fi

# Read current version from Cargo.toml
CURRENT=$(grep '^version' Cargo.toml | head -1 | sed 's/.*"\(.*\)".*/\1/')
IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT"

case "$BUMP_TYPE" in
    major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
    minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
    patch) PATCH=$((PATCH + 1)) ;;
esac

NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"
TAG="v${NEW_VERSION}"
TODAY=$(date +%Y-%m-%d)

echo "=== Releasing $CURRENT → $NEW_VERSION ==="

# Update Cargo.toml
if [[ "$(uname)" == "Darwin" ]]; then
    sed -i '' "s/^version = \"${CURRENT}\"/version = \"${NEW_VERSION}\"/" Cargo.toml
else
    sed -i "s/^version = \"${CURRENT}\"/version = \"${NEW_VERSION}\"/" Cargo.toml
fi

# Update Cargo.lock
cargo check --quiet 2>/dev/null || true

# Update CHANGELOG.md — insert new version heading after [Unreleased]
if [[ "$(uname)" == "Darwin" ]]; then
    sed -i '' "s/^## \[Unreleased\]/## [Unreleased]\n\n## [${TAG}] — ${TODAY}/" CHANGELOG.md
else
    sed -i "s/^## \[Unreleased\]/## [Unreleased]\n\n## [${TAG}] — ${TODAY}/" CHANGELOG.md
fi

# Build all platforms
"$SCRIPT_DIR/build.sh"

# Commit, tag, push
git add Cargo.toml Cargo.lock CHANGELOG.md
git commit -m "chore(release): ${TAG}"
git tag "$TAG"
git push origin main
git push origin "$TAG"

# Create GitHub release with binaries
OUT_DIR="$PROJECT_DIR/build/out"
gh release create "$TAG" \
    --title "$TAG" \
    --generate-notes \
    "$OUT_DIR/gcli-linux-x86_64#gcli-linux-x86_64" \
    "$OUT_DIR/gcli-macos-arm64#gcli-macos-arm64"

echo "=== Released ${TAG} ==="
echo "https://github.com/glennswest/gcli/releases/tag/${TAG}"
