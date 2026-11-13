#!/usr/bin/env bash
set -euo pipefail

# Usage: publish.sh <tag> <artifacts dir>
# Creates the GitHub release for the tag, or adds the files if it already exists.
tag=$1
artifacts=$2
files=$(mktemp -d)

# Binaries are named valser on every target, prefix them with the target
for path in "$artifacts"/valser-*/*; do
    target=$(basename "$(dirname "$path")")
    name=$(basename "$path")
    [[ $name == valser* ]] && name=$target${name#valser}
    mv "$path" "$files/$name"
done

if gh release view "$tag" > /dev/null 2>&1; then
    gh release upload "$tag" "$files"/* --clobber
    exit
fi

# Changelog sections since the last GitHub release, or only this version's if there is none
previous=$(gh release list --exclude-drafts --limit 1 --json tagName --jq '.[0].tagName // ""')
awk -v from="${tag#v}" -v to="${previous#v}" '
    /^## \[\\</ {
        version = $0
        sub(/^## \[\\</, "", version)
        sub(/\\>.*/, "", version)
        if (found && (to == "" || version == to)) exit
        if (version == from) found = 1
    }
    found && !/^---$/
' CHANGELOG.md > "$files.md"

notes=(--notes-file "$files.md")
[[ -s $files.md ]] || notes=(--generate-notes)
gh release create "$tag" --verify-tag --title "Valser ${tag#v}" "${notes[@]}" "$files"/*
