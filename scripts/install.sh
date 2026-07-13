#!/usr/bin/env sh
set -eu

repo="RoTorEx/jumper"
ref="main"
install_dir="${HOME:-}/.x-cli-jumper"
update_profile=1

usage() {
    cat <<'USAGE'
Install jumper from GitHub.

Usage:
  sh scripts/install.sh [--repo owner/name] [--ref ref] [--dir path] [--no-profile]

Environment:
  JUMPER_REPO          GitHub repo, default RoTorEx/jumper
  JUMPER_REF           branch, tag, or commit, default main
  JUMPER_INSTALL_DIR   install directory, default ~/.x-cli-jumper
  GH_INSTALLER_TOKEN   GitHub token for private repo installs

The installer builds with Cargo, copies the jumper binary and shell bridge into
the install directory, and adds one managed source line to bash/zsh profiles.
USAGE
}

repo="${JUMPER_REPO:-$repo}"
ref="${JUMPER_REF:-$ref}"
install_dir="${JUMPER_INSTALL_DIR:-$install_dir}"
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
            shift 2
            ;;
        --dir=*)
            install_dir="${1#--dir=}"
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
if ! command -v tar >/dev/null 2>&1; then
    echo "ERROR: tar is required." >&2
    exit 1
fi

tmp="${TMPDIR:-/tmp}/jumper-install-$$"
archive="$tmp/source.tar.gz"
mkdir -p "$tmp"
trap 'rm -rf "$tmp"' EXIT HUP INT TERM

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

echo "Building jumper"
(cd "$source_dir" && cargo build --release --locked)

mkdir -p "$install_dir"
cp "$source_dir/target/release/jumper" "$install_dir/jumper"
chmod 0755 "$install_dir/jumper"
JUMPER_SHELL_BINARY="$install_dir/jumper" \
    "$install_dir/jumper" --shell-init > "$install_dir/init.zsh"
chmod 0644 "$install_dir/init.zsh"
if [ -n "$installer_token" ]; then
    token_file="$install_dir/gh-token"
    (umask 077 && printf "%s\n" "$installer_token" > "$token_file")
    chmod 0600 "$token_file"
fi

profile_block() {
    init_file="$install_dir/init.zsh"
    escaped_init_file="$(printf '%s' "$init_file" | sed 's/[\\"$`]/\\&/g')"
    printf '[ -r "%s" ] && . "%s"\n' "$escaped_init_file" "$escaped_init_file"
}

remove_existing_block() {
    profile_file="$1"
    cleaned="$tmp/profile-cleaned"
    awk '
        $0 == "# >>> x-cli-jumper >>>" { skip = 1; next }
        $0 == "# <<< x-cli-jumper <<<" { skip = 0; next }
        skip != 1 { print }
    ' "$profile_file" > "$cleaned"
    cat "$cleaned" > "$profile_file"
}

remove_legacy_integration() {
    profile_file="$1"
    cleaned="$tmp/profile-legacy-cleaned"
    awk '
        $0 == "export PATH=\"$HOME/.x-cli-jumper:$PATH\"" { next }
        $0 == "j() {" {
            first = $0
            second = third = fourth = ""
            if ((getline second) > 0 &&
                (getline third) > 0 &&
                (getline fourth) > 0 &&
                second == "    local d" &&
                third == "    d=\"$(jumper \"$@\")\" && [ -n \"$d\" ] && cd \"$d\"" &&
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
    {
        printf "\n# >>> x-cli-jumper >>>\n"
        profile_block
        printf "# <<< x-cli-jumper <<<\n"
    } >> "$profile_file"
    echo "Updated $profile_file"
}

if [ "$update_profile" -eq 1 ]; then
    profiles=""
    [ -f "$HOME/.zshrc" ] && profiles="$profiles $HOME/.zshrc"
    [ -f "$HOME/.bashrc" ] && profiles="$profiles $HOME/.bashrc"

    if [ -z "$profiles" ]; then
        case "${SHELL:-}" in
            */zsh) profiles="$HOME/.zshrc" ;;
            */bash) profiles="$HOME/.bashrc" ;;
            *) profiles="$HOME/.profile" ;;
        esac
    fi

    for profile in $profiles; do
        update_one_profile "$profile"
    done
fi

echo "Installed $install_dir/jumper"
echo "Open a new shell or activate this one with:"
profile_block
