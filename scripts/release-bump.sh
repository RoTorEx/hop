#!/usr/bin/env sh
set -eu

bump="${1:-patch}"

case "$bump" in
    patch|minor|major) ;;
    *)
        echo "ERROR: bump must be patch, minor, or major." >&2
        exit 1
        ;;
esac

current="$(awk -F'"' '/^version = / { print $2; exit }' Cargo.toml)"
if [ -z "$current" ]; then
    echo "ERROR: could not read version from Cargo.toml." >&2
    exit 1
fi

old_ifs="$IFS"
IFS=.
set -- $current
IFS="$old_ifs"

major="${1:-}"
minor="${2:-}"
patch="${3:-}"

case "$major.$minor.$patch" in
    *[!0-9.]*|.*|*..*|*.)
        echo "ERROR: unsupported version: $current" >&2
        exit 1
        ;;
esac

case "$bump" in
    patch)
        patch=$((patch + 1))
        ;;
    minor)
        minor=$((minor + 1))
        patch=0
        ;;
    major)
        major=$((major + 1))
        minor=0
        patch=0
        ;;
esac

next="$major.$minor.$patch"
today="$(date +%Y-%m-%d)"
tmp="${TMPDIR:-/tmp}/jumper-release-bump-$$"
mkdir -p "$tmp"
trap 'rm -rf "$tmp"' EXIT HUP INT TERM

awk -v current="$current" -v next="$next" '
    BEGIN { changed = 0 }
    /^version = / && changed == 0 {
        sub("version = \"" current "\"", "version = \"" next "\"")
        changed = 1
    }
    { print }
' Cargo.toml > "$tmp/Cargo.toml"
cat "$tmp/Cargo.toml" > Cargo.toml

awk -v version="$next" -v today="$today" '
    /^## \[Unreleased\]$/ {
        print
        print ""
        print "## [" version "] - " today
        next
    }
    { print }
' CHANGELOG.md > "$tmp/CHANGELOG.md"
cat "$tmp/CHANGELOG.md" > CHANGELOG.md

cargo generate-lockfile

echo "Bumped jumper from $current to $next"
echo "Review, commit, tag with make release-tag, then publish with make release-publish."
