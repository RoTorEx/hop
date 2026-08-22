#!/usr/bin/env sh
set -eu

repo="RoTorEx/hop"
ref="main"
install_dir="${HOME:-}/.x-cli-hop"
bin_dir="$install_dir/bin"
update_profile=1
source_dir=""

usage() {
    cat <<'USAGE'
Install hop from GitHub.

Usage:
  sh scripts/install.sh [--repo owner/name] [--ref ref] [--dir path] [--no-profile]
  sh scripts/install.sh --source path [--dir path] [--no-profile]

Environment:
  HOP_REPO          GitHub repo, default RoTorEx/hop
  HOP_REF           branch, tag, or commit, default main
  HOP_INSTALL_DIR   install directory, default ~/.x-cli-hop
  GH_INSTALLER_TOKEN   GitHub token for private repo installs

The installer builds with Cargo, installs the binary under bin/, writes the
shell bridge, and adds one source line to the active bash/zsh profile.

Use --source to build from an existing local checkout instead of GitHub.
USAGE
}

repo="${HOP_REPO:-$repo}"
ref="${HOP_REF:-$ref}"
install_dir="${HOP_INSTALL_DIR:-$install_dir}"
bin_dir="$install_dir/bin"
installer_token="${GH_INSTALLER_TOKEN:-}"

while [ "$#" -gt 0 ]; do
    case "$1" in
        --repo)
            if [ "$#" -lt 2 ]; then
                echo "ERROR: --repo requires a value." >&2
                exit 1
            fi
            repo="${2:-}"
            shift 2
            ;;
        --repo=*)
            repo="${1#--repo=}"
            shift
            ;;
        --ref)
            if [ "$#" -lt 2 ]; then
                echo "ERROR: --ref requires a value." >&2
                exit 1
            fi
            ref="${2:-}"
            shift 2
            ;;
        --ref=*)
            ref="${1#--ref=}"
            shift
            ;;
        --dir)
            if [ "$#" -lt 2 ]; then
                echo "ERROR: --dir requires a value." >&2
                exit 1
            fi
            install_dir="${2:-}"
            bin_dir="$install_dir/bin"
            shift 2
            ;;
        --dir=*)
            install_dir="${1#--dir=}"
            bin_dir="$install_dir/bin"
            shift
            ;;
        --source)
            if [ "$#" -lt 2 ]; then
                echo "ERROR: --source requires a value." >&2
                exit 1
            fi
            source_dir="${2:-}"
            shift 2
            ;;
        --source=*)
            source_dir="${1#--source=}"
            shift
            ;;
        --no-profile)
            update_profile=0
            shift
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "ERROR: unknown argument: $1" >&2
            usage >&2
            exit 1
            ;;
    esac
done

if [ -z "${HOME:-}" ]; then
    echo "ERROR: HOME is not set." >&2
    exit 1
fi
if [ -z "$repo" ] || [ -z "$ref" ] || [ -z "$install_dir" ]; then
    echo "ERROR: repo, ref, and install directory must not be empty." >&2
    exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
    echo "ERROR: cargo is required. Install Rust from https://rustup.rs and run again." >&2
    exit 1
fi
if [ -z "$source_dir" ] && ! command -v tar >/dev/null 2>&1; then
    echo "ERROR: tar is required." >&2
    exit 1
fi

tmp="${TMPDIR:-/tmp}/hop-install-$$"
archive="$tmp/source.tar.gz"
mkdir -p "$tmp"
trap 'rm -rf "$tmp"' EXIT HUP INT TERM

if [ -n "$source_dir" ]; then
    if [ ! -f "$source_dir/Cargo.toml" ]; then
        echo "ERROR: local source does not contain Cargo.toml: $source_dir" >&2
        exit 1
    fi
else
    url="https://github.com/$repo/archive/$ref.tar.gz"
    echo "Downloading $url"
    if [ -n "$installer_token" ]; then
        if ! command -v curl >/dev/null 2>&1; then
            echo "ERROR: curl is required for authenticated installs." >&2
            exit 1
        fi
        {
            printf 'fail\n'
            printf 'show-error\n'
            printf 'silent\n'
            printf 'location\n'
            printf 'url = "%s"\n' "$url"
            printf 'output = "%s"\n' "$archive"
            printf 'header = "Authorization: Bearer %s"\n' "$installer_token"
        } | curl -K -
    elif command -v curl >/dev/null 2>&1; then
        curl -fsSL "$url" -o "$archive"
    elif command -v wget >/dev/null 2>&1; then
        wget -qO "$archive" "$url"
    else
        echo "ERROR: curl or wget is required." >&2
        exit 1
    fi

    tar -xzf "$archive" -C "$tmp"
    source_dir="$(find "$tmp" -mindepth 1 -maxdepth 1 -type d | head -n 1)"
    if [ -z "$source_dir" ]; then
        echo "ERROR: could not unpack source archive." >&2
        exit 1
    fi
fi

echo "Building hop"
build_target_dir="$tmp/target"
(cd "$source_dir" && CARGO_TARGET_DIR="$build_target_dir" cargo build --release --locked)
build_binary="$build_target_dir/release/hop"

installed_binary="$bin_dir/hop"
legacy_binary="$install_dir/hop"
generated_init="$tmp/init.zsh"
temporary_binary="$bin_dir/.hop-install-$$"
temporary_init="$install_dir/.init.zsh-install-$$"

mkdir -p "$bin_dir"
HOP_SHELL_BINARY="$installed_binary" \
    "$build_binary" --shell-init > "$generated_init"
cp "$build_binary" "$temporary_binary"
chmod 0755 "$temporary_binary"
mv "$temporary_binary" "$installed_binary"
cp "$generated_init" "$temporary_init"
chmod 0644 "$temporary_init"
mv "$temporary_init" "$install_dir/init.zsh"
if [ -f "$legacy_binary" ]; then
    rm -f "$legacy_binary"
fi
if [ -n "$installer_token" ]; then
    token_file="$install_dir/gh-token"
    (umask 077 && printf "%s\n" "$installer_token" > "$token_file")
    chmod 0600 "$token_file"
fi

legacy_path_export() {
    escaped_install_dir="$(printf '%s' "$install_dir" | sed 's/[\\"$`]/\\&/g')"
    if [ "$install_dir" = "$HOME/.x-cli-hop" ]; then
        printf 'export PATH="$HOME/.x-cli-hop:$PATH"\n'
    else
        printf 'export PATH="%s:$PATH"\n' "$escaped_install_dir"
    fi
}

legacy_bin_path_export() {
    escaped_bin_dir="$(printf '%s' "$bin_dir" | sed 's/[\\"$`]/\\&/g')"
    if [ "$bin_dir" = "$HOME/.x-cli-hop/bin" ]; then
        printf 'export PATH="$HOME/.x-cli-hop/bin:$PATH"\n'
    else
        printf 'export PATH="%s:$PATH"\n' "$escaped_bin_dir"
    fi
}

profile_source() {
    init_file="$install_dir/init.zsh"
    escaped_init_file="$(printf '%s' "$init_file" | sed 's/[\\"$`]/\\&/g')"
    if [ "$init_file" = "$HOME/.x-cli-hop/init.zsh" ]; then
        printf 'source "$HOME/.x-cli-hop/init.zsh"\n'
    else
        printf 'source "%s"\n' "$escaped_init_file"
    fi
}

legacy_profile_source() {
    init_file="$install_dir/init.zsh"
    escaped_init_file="$(printf '%s' "$init_file" | sed 's/[\\"$`]/\\&/g')"
    printf '[ -r "%s" ] && . "%s"\n' "$escaped_init_file" "$escaped_init_file"
}

ensure_profile_line() {
    profile_file="$1"
    profile_line="$2"
    if ! grep -Fqx "$profile_line" "$profile_file"; then
        printf '\n%s\n' "$profile_line" >> "$profile_file"
    fi
}

remove_existing_block() {
    profile_file="$1"
    cleaned="$tmp/profile-cleaned"
    awk '
        $0 == "# >>> x-cli-hop >>>" { skip = 1; next }
        $0 == "# <<< x-cli-hop <<<" { skip = 0; next }
        skip != 1 { print }
    ' "$profile_file" > "$cleaned"
    cat "$cleaned" > "$profile_file"
}

remove_legacy_integration() {
    profile_file="$1"
    cleaned="$tmp/profile-legacy-cleaned"
    old_source_line="$(legacy_profile_source)"
    old_path_line="$(legacy_path_export)"
    old_bin_path_line="$(legacy_bin_path_export)"
    awk \
        -v old_source_line="$old_source_line" \
        -v old_path_line="$old_path_line" \
        -v old_bin_path_line="$old_bin_path_line" '
        $0 == "# x-cli-hop" { next }
        $0 == old_source_line { next }
        $0 == old_path_line { next }
        $0 == old_bin_path_line { next }
        $0 == "j() {" {
            first = $0
            second = third = fourth = ""
            if ((getline second) > 0 &&
                (getline third) > 0 &&
                (getline fourth) > 0 &&
                second == "    local d" &&
                third == "    d=\"$(hop \"$@\")\" && [ -n \"$d\" ] && cd \"$d\"" &&
                fourth == "}") {
                next
            }
            print first
            if (second != "") print second
            if (third != "") print third
            if (fourth != "") print fourth
            next
        }
        { print }
    ' "$profile_file" > "$cleaned"
    cat "$cleaned" > "$profile_file"
}

update_one_profile() {
    profile_file="$1"
    mkdir -p "$(dirname "$profile_file")"
    touch "$profile_file"
    remove_legacy_integration "$profile_file"
    remove_existing_block "$profile_file"
    ensure_profile_line "$profile_file" "$(profile_source)"
    echo "Updated $profile_file"
}

if [ "$update_profile" -eq 1 ]; then
    profile=""
    case "${SHELL:-}" in
        */zsh) profile="$HOME/.zshrc" ;;
        */bash) profile="$HOME/.bashrc" ;;
        *)
            echo "Shell integration supports bash and zsh; profile was not changed." >&2
            ;;
    esac

    if [ -n "$profile" ]; then
        update_one_profile "$profile"
    fi
fi

echo "Installed $installed_binary"
echo "Open a new shell or activate this one with:"
profile_source
