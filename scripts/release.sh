#!/usr/bin/env sh
set -eu

bump="${1:-patch}"

usage() {
    cat <<'USAGE'
Run the guarded jumper release flow.

Usage:
  make release [BUMP=current|patch|minor|major]

Environment:
  APPROVE_RELEASE=1    Skip the interactive confirmation prompt.

Flow:
  1. Require main and a clean working tree.
  2. Fetch origin/main and tags.
  3. Run make check.
  4. Run make release-bump.
  5. Run make check again.
  6. Commit version/changelog metadata.
  7. Create vX.Y.Z tag.
  8. Push main and tags with --follow-tags.

Use BUMP=current for the first release when Cargo.toml already contains the
version to tag. Future releases default to BUMP=patch.
USAGE
}

fail() {
    echo "ERROR: $*" >&2
    exit 1
}

read_version() {
    awk -F'"' '/^version = / { print $2; exit }' Cargo.toml
}

next_version() {
    current="$1"
    release_bump="$2"

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

    case "$release_bump" in
        current)
            ;;
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
        *)
            fail "bump must be current, patch, minor, or major"
            ;;
    esac

    printf "%s.%s.%s\n" "$major" "$minor" "$patch"
}

confirm_release() {
    target="$1"

    if [ "${APPROVE_RELEASE:-}" = "1" ]; then
        return 0
    fi

    echo "Release approval required."
    echo "This will create a release commit, create tag v$target, and push main with tags."
    printf "Type 'release v%s' to continue: " "$target"
    read -r answer
    [ "$answer" = "release v$target" ] || fail "release cancelled"
}

case "$bump" in
    -h|--help)
        usage
        exit 0
        ;;
    current|patch|minor|major)
        ;;
    *)
        fail "bump must be current, patch, minor, or major"
        ;;
esac

branch="$(git rev-parse --abbrev-ref HEAD)"
[ "$branch" = "main" ] || fail "release must run from main, not $branch"

[ -z "$(git status --porcelain)" ] || fail "commit or stash changes before releasing"

current="$(read_version)"
[ -n "$current" ] || fail "could not read version from Cargo.toml"
target="$(next_version "$current" "$bump")"
tag="v$target"

confirm_release "$target"

git fetch origin main --tags

if git rev-parse --verify "refs/tags/$tag" >/dev/null 2>&1; then
    fail "tag $tag already exists"
fi

if ! git merge-base --is-ancestor origin/main HEAD; then
    fail "local main is behind or diverged from origin/main"
fi

make check
make release-bump BUMP="$bump"

target_after_bump="$(read_version)"
[ "$target_after_bump" = "$target" ] || fail "expected Cargo.toml version $target, got $target_after_bump"

make check

git add Cargo.toml Cargo.lock CHANGELOG.md

if git diff --cached --quiet; then
    fail "release produced no commit-worthy metadata changes"
fi

if [ "$bump" = "current" ]; then
    git commit -m "build: release v$target"
else
    git commit -m "build: bump version to v$target"
fi

make release-tag
make release-publish

echo "Released $tag"
