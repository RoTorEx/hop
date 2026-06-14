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
  3. Prompt for the exact MAJOR.MINOR.PATCH version.
  4. Run make check.
  5. Update release metadata for that exact version.
  6. Run make check again.
  7. Commit release metadata.
  8. Create vX.Y.Z tag.

Run make release-push after review to push main and tags.
USAGE
}

fail() {
    echo "ERROR: $*" >&2
    exit 1
}

read_version() {
    awk -F'"' '/^version = / { print $2; exit }' Cargo.toml
}

validate_version() {
    version="$1"

    old_ifs="$IFS"
    IFS=.
    set -- $version
    IFS="$old_ifs"

    [ "$#" -eq 3 ] || fail "version must be MAJOR.MINOR.PATCH"

    case "${1:-}" in ''|*[!0-9]*) fail "version must be MAJOR.MINOR.PATCH" ;; esac
    case "${2:-}" in ''|*[!0-9]*) fail "version must be MAJOR.MINOR.PATCH" ;; esac
    case "${3:-}" in ''|*[!0-9]*) fail "version must be MAJOR.MINOR.PATCH" ;; esac
}

apply_release_version() {
    current="$1"
    target="$2"
    today="$(date +%Y-%m-%d)"
    tmp="${TMPDIR:-/tmp}/jumper-release-$$"
    mkdir -p "$tmp"

    if grep -q "^## \[$target\]" CHANGELOG.md; then
        fail "CHANGELOG.md already has a $target release section"
    fi

    awk -v current="$current" -v target="$target" '
        BEGIN { changed = 0 }
        /^version = / && changed == 0 {
            sub("version = \"" current "\"", "version = \"" target "\"")
            changed = 1
        }
        { print }
    ' Cargo.toml > "$tmp/Cargo.toml"
    cat "$tmp/Cargo.toml" > Cargo.toml

    awk -v version="$target" -v today="$today" '
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

current="$(read_version)"
[ -n "$current" ] || fail "could not read version from Cargo.toml"

printf "Release version (MAJOR.MINOR.PATCH): "
read -r target
[ -n "$target" ] || fail "release version is required"
validate_version "$target"

tag="v$target"
if git rev-parse --verify "refs/tags/$tag" >/dev/null 2>&1; then
    fail "tag $tag already exists"
fi

make check
apply_release_version "$current" "$target"
make check

git add Cargo.toml Cargo.lock CHANGELOG.md

if git diff --cached --quiet; then
    fail "release produced no commit-worthy metadata changes"
fi

git commit -m "build: release v$target"
make release-tag

echo "Prepared $tag"
echo "Run: make release-push"
