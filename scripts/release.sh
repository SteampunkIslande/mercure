#!/usr/bin/env bash
#
# Bump the project version (semver), update Cargo.toml + Cargo.lock,
# commit, tag, push, and trigger the release workflow.
#
# Usage:
#   ./scripts/release.sh            # interactive: ask major/minor/patch
#   ./scripts/release.sh major      # non-interactive bump
#   ./scripts/release.sh minor
#   ./scripts/release.sh patch
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/.." && pwd)"
cd "$REPO_ROOT"

# ---------------------------------------------------------------------------
# 1. Choose bump type (interactive if not provided)
# ---------------------------------------------------------------------------
BUMP="${1:-}"
if [[ -z "$BUMP" ]]; then
  echo "Choisissez le type d'incrementation de la version :"
  echo "  1) patch  (x.y.Z  -> corrections / bug fixes)"
  echo "  2) minor  (x.Y.z  -> nouvelle fonctionnalite, API stable)"
  echo "  3) major  (X.y.z  -> changements majeurs, API cassante)"
  read -r CHOICE </dev/tty
  case "$CHOICE" in
    1) BUMP="patch" ;;
    2) BUMP="minor" ;;
    3) BUMP="major" ;;
    *) echo "Choix invalide: '$CHOICE'" >&2; exit 1 ;;
  esac
fi

if [[ ! "$BUMP" =~ ^(major|minor|patch)$ ]]; then
  echo "Invalid bump type: '$BUMP'. Use major, minor, or patch." >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# 2. Read current version from Cargo.toml
# ---------------------------------------------------------------------------
CURRENT_VERSION="$(awk '/^\[package\]/{p=1} p && /^version/{gsub(/"/,"",$3); print $3; exit}' Cargo.toml)"
if [[ -z "$CURRENT_VERSION" ]]; then
  echo "Could not read version from Cargo.toml" >&2
  exit 1
fi
echo "Current version: $CURRENT_VERSION"

IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"
MAJOR="${MAJOR:-0}"
MINOR="${MINOR:-0}"
PATCH="${PATCH:-0}"

case "$BUMP" in
  major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
  minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
  patch) PATCH=$((PATCH + 1)) ;;
esac

NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"
echo "New version: $NEW_VERSION"

# ---------------------------------------------------------------------------
# 3. Update Cargo.toml (only the [package] version line)
# ---------------------------------------------------------------------------
awk -v new="$NEW_VERSION" '
  /^\[package\]/{ in_pkg=1; print; next }
  in_pkg && /^\[/ { in_pkg=0 }
  in_pkg && /^version *= */ {
    sub(/version *= *"[^"]*"/, "version = \"" new "\"")
  }
  { print }
' Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml

# ---------------------------------------------------------------------------
# 4. Update Cargo.lock
# ---------------------------------------------------------------------------
cargo update -p smaug

UPDATED_VERSION="$(awk '/^\[package\]/{p=1} p && /^version/{gsub(/"/,"",$3); print $3; exit}' Cargo.toml)"
if [[ "$UPDATED_VERSION" != "$NEW_VERSION" ]]; then
  echo "Version update failed: expected $NEW_VERSION, got $UPDATED_VERSION" >&2
  exit 1
fi

# ---------------------------------------------------------------------------
# 5. Commit, tag and push
# ---------------------------------------------------------------------------
TAG="v$NEW_VERSION"
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to $NEW_VERSION ($BUMP)"
git tag "$TAG"

git push origin main
git push origin "$TAG"

echo ""
echo "Version bumped to $NEW_VERSION and tagged as $TAG."

# ---------------------------------------------------------------------------
# 6. Trigger the release workflow (workflow_dispatch)
# ---------------------------------------------------------------------------
REPO="$(git config --get remote.origin.url | sed -E 's#git@([^:]+):([^/]+)/([^/]+)\.git#https://\1/\2/\3#' | sed -E 's#git@([^:]+):#\1/#')"
if [[ -z "$REPO" || "$REPO" == "git@"* ]]; then
  REPO="$(git remote get-url origin | sed -E 's#git@([^:]+):#\1/#; s#\.git$##')"
fi
echo "Repository: $REPO"

TRIGGERED=0
if command -v gh >/dev/null 2>&1 && gh auth status >/dev/null 2>&1; then
  echo "Triggering release via gh..."
  gh workflow run release.yml --repo "$REPO" --ref main -- field tag_name="$TAG" 2>/dev/null \
    || gh workflow run release.yml --repo "$REPO" --ref main
  TRIGGERED=1
elif [[ -n "${GITHUB_TOKEN:-}${GH_TOKEN:-}" ]]; then
  TOKEN="${GITHUB_TOKEN:-$GH_TOKEN}"
  echo "Triggering release via API..."
  curl -sS -X POST \
    -H "Authorization: token $TOKEN" \
    -H "Accept: application/vnd.github+json" \
    "$REPO/actions/workflows/release.yml/dispatches" \
    -d "{\"ref\":\"main\",\"inputs\":{\"tag_name\":\"$TAG\"}}" >/dev/null
  TRIGGERED=1
fi

if [[ "$TRIGGERED" -eq 1 ]]; then
  echo "Release workflow triggered for tag $TAG."
else
  echo "Could not trigger the workflow automatically."
  echo "Start it manually at: $REPO/actions/workflows/release.yml"
fi