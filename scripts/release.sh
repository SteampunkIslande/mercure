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
cd "$(dirname "${BASH_SOURCE[0]}")/.."

# --- 1. Choose bump type ----------------------------------------------------
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
[[ "$BUMP" =~ ^(major|minor|patch)$ ]] || { echo "Invalid bump type: '$BUMP'" >&2; exit 1; }

# --- 2. Compute new version -------------------------------------------------
CURRENT_VERSION="$(awk '/^\[package\]/{p=1} p && /^version/{gsub(/"/,"",$3); print $3; exit}' Cargo.toml)"
[[ -n "$CURRENT_VERSION" ]] || { echo "Could not read version from Cargo.toml" >&2; exit 1; }
echo "Current version: $CURRENT_VERSION"

IFS='.' read -r MAJOR MINOR PATCH <<< "$CURRENT_VERSION"
MAJOR="${MAJOR:-0}"; MINOR="${MINOR:-0}"; PATCH="${PATCH:-0}"
case "$BUMP" in
  major) MAJOR=$((MAJOR + 1)); MINOR=0; PATCH=0 ;;
  minor) MINOR=$((MINOR + 1)); PATCH=0 ;;
  patch) PATCH=$((PATCH + 1)) ;;
esac
NEW_VERSION="${MAJOR}.${MINOR}.${PATCH}"
echo "New version: $NEW_VERSION"

# --- 3. Update Cargo.toml + Cargo.lock -------------------------------------
awk -v new="$NEW_VERSION" '
  /^\[package\]/{ in_pkg=1; print; next }
  in_pkg && /^\[/ { in_pkg=0 }
  in_pkg && /^version *= */ { sub(/version *= *"[^"]*"/, "version = \"" new "\"") }
  { print }
' Cargo.toml > Cargo.toml.tmp && mv Cargo.toml.tmp Cargo.toml

cargo update -p smaug

UPDATED_VERSION="$(awk '/^\[package\]/{p=1} p && /^version/{gsub(/"/,"",$3); print $3; exit}' Cargo.toml)"
[[ "$UPDATED_VERSION" == "$NEW_VERSION" ]] || { echo "Version update failed: expected $NEW_VERSION, got $UPDATED_VERSION" >&2; exit 1; }

# --- 4. Commit, tag, push ---------------------------------------------------
TAG="v$NEW_VERSION"
git add Cargo.toml Cargo.lock
git commit -m "chore: bump version to $NEW_VERSION ($BUMP)"
git tag "$TAG"
git push origin main "$TAG"
echo "Version bumped to $NEW_VERSION and tagged as $TAG."

# --- 5. Trigger the release workflow ---------------------------------------
gh workflow run release.yml --ref main --field tag_name="$TAG"
echo "Release workflow triggered for tag $TAG."