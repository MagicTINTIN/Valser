#!/usr/bin/env bash
set -euo pipefail

# Usage:
#   ./release.sh patch
#   ./release.sh minor
#   ./release.sh major

LEVEL="${1:-}"

if [[ ! "$LEVEL" =~ ^(patch|minor|major)$ ]]; then
  echo "Usage: $0 {patch|minor|major}"
  exit 1
fi

# Ensure working tree is clean
git diff --quiet && git diff --cached --quiet || {
  echo "Working tree is not clean."
  exit 1
}

git cliff -o CHANGELOG.md

cargo release "$LEVEL" \
  --execute \
  --no-publish \
  --no-push

echo "✔ Release complete"