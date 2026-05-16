#!/usr/bin/env sh
set -eu

usage() {
    cat <<'USAGE'
Run the jumper release flow.

Usage:
  make release

Flow:
  1. Require main and a clean working tree.
  2. Fetch origin/main and tags.
  3. Run make check.
  4. Finalize the current version if it has no tag yet; otherwise bump patch.
  5. Run make check again.
  6. Commit release metadata.
  7. Create vX.Y.Z tag.
  8. Push main and tags with --follow-tags.
USAGE
}

fail() {
    echo "ERROR: $*" >&2
    exit 1
}

read_version() {
    awk -F'"' '/^version = / { print $2; exit }' Cargo.toml
}

finalize_current_version() {
    version="$1"
    today="$(date +%Y-%m-%d)"
    tmp="${TMPDIR:-/tmp}/jumper-release-$$"
    mkdir -p "$tmp"

    if grep -q "^## \[$version\]" CHANGELOG.md; then
        fail "CHANGELOG.md already has a $version release section"
    fi

    awk -v version="$version" -v today="$today" '
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
}

case "${1:-}" in
    "")
        ;;
    -h|--help)
        usage
        exit 0
        ;;
    *)
        fail "make release does not accept release flags"
        ;;
esac

trap 'rm -rf "${TMPDIR:-/tmp}/jumper-release-$$"' EXIT HUP INT TERM

branch="$(git rev-parse --abbrev-ref HEAD)"
[ "$branch" = "main" ] || fail "release must run from main, not $branch"

[ -z "$(git status --porcelain)" ] || fail "commit or stash changes before releasing"

git fetch origin main --tags

if ! git merge-base --is-ancestor origin/main HEAD; then
    fail "local main is behind or diverged from origin/main"
fi

make check

current="$(read_version)"
[ -n "$current" ] || fail "could not read version from Cargo.toml"
current_tag="v$current"

if git rev-parse --verify "refs/tags/$current_tag" >/dev/null 2>&1; then
    make release-bump
    target="$(read_version)"
    commit_message="build: bump version to v$target"
else
    finalize_current_version "$current"
    target="$current"
    commit_message="build: release v$target"
fi

tag="v$target"
if git rev-parse --verify "refs/tags/$tag" >/dev/null 2>&1; then
    fail "tag $tag already exists"
fi

make check

git add Cargo.toml Cargo.lock CHANGELOG.md

if git diff --cached --quiet; then
    fail "release produced no commit-worthy metadata changes"
fi

git commit -m "$commit_message"
make release-tag
make release-publish

echo "Released $tag"
