#!/usr/bin/env bash
set -euo pipefail

version=$(git cliff --bumped-version 2>/dev/null)
cargo release "${version#v}" --workspace --execute "$@"
