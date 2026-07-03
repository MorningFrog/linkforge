#!/usr/bin/env sh

set -eu

usage() {
    cat <<'EOF'
Usage: scripts/set-version.sh <version> [--dry-run] [--no-lock]

Updates workspace package versions, local path dependency versions, the
Tauri app version, and the Windows sparse-package manifest version.

Options:
  --dry-run   Show files that would be updated without changing them.
  --no-lock   Do not refresh Cargo.lock.
EOF
}

if [ "$#" -lt 1 ]; then
    usage
    exit 2
fi

version=$1
shift
dry_run=0
no_lock=0

while [ "$#" -gt 0 ]; do
    case "$1" in
        --dry-run)
            dry_run=1
            ;;
        --no-lock)
            no_lock=1
            ;;
        -h|--help)
            usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1" >&2
            usage >&2
            exit 2
            ;;
    esac
    shift
done

case "$version" in
    *[!0-9A-Za-z.+-]* | "" | .* | *..* | *+*+*)
        echo "Invalid version '$version'. Expected SemVer, for example: 1.2.3, 1.2.3-beta.1, or 1.2.3+build.4." >&2
        exit 2
        ;;
esac

if ! printf '%s\n' "$version" | awk '
    /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(-[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?(\+[0-9A-Za-z-]+(\.[0-9A-Za-z-]+)*)?$/ { ok = 1 }
    END { exit ok ? 0 : 1 }
'; then
    echo "Invalid version '$version'. Expected SemVer, for example: 1.2.3, 1.2.3-beta.1, or 1.2.3+build.4." >&2
    exit 2
fi

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo_root=$(CDPATH= cd -- "$script_dir/.." && pwd)

if ! command -v cargo >/dev/null 2>&1; then
    echo "cargo was not found in PATH." >&2
    exit 1
fi

cd "$repo_root"

tmp_dir=${TMPDIR:-/tmp}
manifests_file=$(mktemp "$tmp_dir/linkforge-manifests.XXXXXX")
names_file=$(mktemp "$tmp_dir/linkforge-package-names.XXXXXX")
trap 'rm -f "$manifests_file" "$names_file"; if [ -n "$manifest_tmp" ]; then rm -f "$manifest_tmp"; fi' EXIT HUP INT TERM
manifest_tmp=

find . \
    -path './target' -prune -o \
    -path './.git' -prune -o \
    -name Cargo.toml -type f -print |
    sort > "$manifests_file"

while IFS= read -r manifest; do
    awk '
        /^\[package\]$/ { in_package = 1; next }
        /^\[/ { in_package = 0 }
        in_package && /^[[:space:]]*name[[:space:]]*=/ {
            line = $0
            sub(/^[^"]*"/, "", line)
            sub(/".*$/, "", line)
            print line
            exit
        }
    ' "$manifest"
done < "$manifests_file" | sort -u > "$names_file"

if [ ! -s "$names_file" ]; then
    echo "No workspace packages found." >&2
    exit 1
fi

changed=0

appx_version=$(printf '%s\n' "$version" | sed -E 's/^([0-9]+)\.([0-9]+)\.([0-9]+).*/\1.\2.\3.0/')

update_if_changed() {
    path=$1
    updated=$2

    if ! cmp -s "$path" "$updated"; then
        changed=1
        if [ "$dry_run" -eq 1 ]; then
            echo "Would update $path"
        else
            mv "$updated" "$path"
            echo "Updated $path"
        fi
    fi

    if [ -e "$updated" ]; then
        rm -f "$updated"
    fi
}

while IFS= read -r manifest; do
    if ! awk '
        /^\[package\]$/ { found = 1 }
        END { exit found ? 0 : 1 }
    ' "$manifest"; then
        continue
    fi

    manifest_tmp=$(mktemp "$tmp_dir/linkforge-manifest.XXXXXX")

    awk -v new_version="$version" -v names_file="$names_file" '
        BEGIN {
            while ((getline name < names_file) > 0) {
                package_names[name] = 1
            }
            close(names_file)
        }
        function replace_version(line) {
            sub(/version[[:space:]]*=[[:space:]]*"[^"]+"/, "version = \"" new_version "\"", line)
            return line
        }
        /^\[package\]$/ {
            in_package = 1
            package_version_done = 0
            print
            next
        }
        /^\[/ && $0 !~ /^\[package\]$/ {
            in_package = 0
        }
        in_package && !package_version_done && /^[[:space:]]*version[[:space:]]*=/ {
            sub(/"[^"]+"/, "\"" new_version "\"")
            package_version_done = 1
            print
            next
        }
        /^[[:space:]]*[A-Za-z0-9_-]+[[:space:]]*=/ && /path[[:space:]]*=/ {
            dep_name = $0
            sub(/^[[:space:]]*/, "", dep_name)
            sub(/[[:space:]]*=.*/, "", dep_name)

            if ((dep_name in package_names) && $0 ~ /version[[:space:]]*=/) {
                print replace_version($0)
                next
            }
        }
        { print }
    ' "$manifest" > "$manifest_tmp"

    if ! cmp -s "$manifest" "$manifest_tmp"; then
        update_if_changed "$manifest" "$manifest_tmp"
        manifest_tmp=
    fi

    if [ -n "$manifest_tmp" ]; then
        rm -f "$manifest_tmp"
    fi
    manifest_tmp=
done < "$manifests_file"

tauri_config=./crates/linkforge-gui/tauri.conf.json
manifest_tmp=$(mktemp "$tmp_dir/linkforge-tauri-conf.XXXXXX")
awk -v new_version="$version" '
    !done && /"version"[[:space:]]*:/ {
        sub(/"version"[[:space:]]*:[[:space:]]*"[^"]+"/, "\"version\": \"" new_version "\"")
        done = 1
    }
    { print }
' "$tauri_config" > "$manifest_tmp"
update_if_changed "$tauri_config" "$manifest_tmp"
manifest_tmp=

windows_register_script=./scripts/context-menu/windows/modern/Register-LinkForgeModernContextMenu.ps1
manifest_tmp=$(mktemp "$tmp_dir/linkforge-windows-register.XXXXXX")
awk -v appx_version="$appx_version" '
    /^\$SparsePackageVersion[[:space:]]*=/ {
        print "$SparsePackageVersion = \"" appx_version "\""
        next
    }
    { print }
' "$windows_register_script" > "$manifest_tmp"
update_if_changed "$windows_register_script" "$manifest_tmp"
manifest_tmp=

if [ "$changed" -eq 0 ]; then
    echo "All release version files already use version $version."
fi

if [ "$dry_run" -eq 1 ]; then
    echo "Dry run only; no files were changed."
elif [ "$no_lock" -eq 0 ]; then
    cargo generate-lockfile --manifest-path "$repo_root/Cargo.toml"
    echo "Refreshed Cargo.lock"
fi
