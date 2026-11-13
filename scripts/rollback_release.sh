#!/usr/bin/env bash
# Atomically point ROOT/current back at ROOT/previous.
set -euo pipefail

[[ $# -eq 1 ]] || { echo "usage: $0 ROOT" >&2; exit 2; }
root=$(realpath -m "$1")
[[ -f "$root/previous" ]] || { echo "no previous release" >&2; exit 1; }
target=$(cat "$root/previous")
[[ -f "$target/places.idx" ]] || { echo "previous release is incomplete" >&2; exit 1; }
current=$(readlink -f "$root/current" 2>/dev/null || true)
tmp="$root/.current.$$"
ln -s "$target" "$tmp"
mv -Tf "$tmp" "$root/current"
echo "$current" > "$root/.previous.tmp"
mv -f "$root/.previous.tmp" "$root/previous"
echo "rolled back to $target"
