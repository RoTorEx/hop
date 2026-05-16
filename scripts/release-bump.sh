#!/usr/bin/env sh
set -eu

fail() {
    echo "ERROR: $*" >&2
    exit 1
}

current="$(awk -F'"' '/^version = / { print $2; exit }' Cargo.toml)"
[ -n "$current" ] || fail "could not read version from Cargo.toml"

old_ifs="$IFS"
IFS=.
set -- $current
IFS="$old_ifs"

major="${1:-}"
minor="${2:-}"
patch="${3:-}"

case "$major" in ''|*[!0-9]*) fail "unsupported version: $current" ;; esac
case "$minor" in ''|*[!0-9]*) fail "unsupported version: $current" ;; esac
case "$patch" in ''|*[!0-9]*) fail "unsupported version: $current" ;; esac

patch=$((patch + 1))
next="$major.$minor.$patch"
today="$(date +%Y-%m-%d)"
tmp="${TMPDIR:-/tmp}/jumper-release-bump-$$"
mkdir -p "$tmp"
trap 'rm -rf "$tmp"' EXIT HUP INT TERM

if grep -q "^## \[$next\]" CHANGELOG.md; then
    fail "CHANGELOG.md already has a $next release section"
fi

awk -v current="$current" -v target="$next" '
    BEGIN { changed = 0 }
    /^version = / && changed == 0 {
        sub("version = \"" current "\"", "version = \"" target "\"")
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
