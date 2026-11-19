#!/usr/bin/env bash
# Validate a release directory and atomically point ROOT/current at it.
set -euo pipefail

usage() {
    echo "usage: $0 RELEASE_DIR ROOT" >&2
    exit 2
}

[[ $# -eq 2 ]] || usage
release=$(realpath "$1")
root=$(realpath -m "$2")
[[ -f "$release/places.idx" ]] || { echo "missing places.idx" >&2; exit 1; }
[[ -f "$release/places.idx.json" ]] || { echo "missing manifest" >&2; exit 1; }
[[ -f "$release/places.idx.sha256" ]] || { echo "missing checksum file" >&2; exit 1; }

expected=$(cut -d' ' -f1 "$release/places.idx.sha256")
actual=$(sha256sum "$release/places.idx" | cut -d' ' -f1)
[[ "$expected" == "$actual" ]] || { echo "checksum mismatch" >&2; exit 1; }
jq -e '(.format_version == 1 or .format_version == 2) and .counts.features > 0' "$release/places.idx.json" >/dev/null

mkdir -p "$root"
if [[ -L "$root/current" ]]; then
    readlink -f "$root/current" > "$root/.previous.tmp"
    mv -f "$root/.previous.tmp" "$root/previous"
fi
tmp="$root/.current.$$"
ln -s "$release" "$tmp"
mv -Tf "$tmp" "$root/current"
echo "promoted $release"
