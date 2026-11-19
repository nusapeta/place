#!/usr/bin/env bash
# Publish data artifacts to a GitHub release. Run from the repo root after the
# artifacts have been verified locally.
#
#   scripts/publish_data.sh data-2026.11 ../artifacts/place-data-2026.11/*
#
# Every upload gets a sibling .sha256. PROVENANCE.txt and LICENSE-DATA are
# picked up from the directory of the first artifact so a release never ships
# data without its source, build command, and licence.
set -euo pipefail

tag=${1:-}
if [[ -z $tag || $# -lt 2 ]]; then
  echo "usage: $0 <data-YYYY.MM> <file>..." >&2
  exit 2
fi
shift

if [[ $tag != data-* ]]; then
  echo "refusing tag '$tag': data releases must start with data-" >&2
  exit 2
fi

for f in "$@"; do
  [[ -f $f ]] || { echo "missing file: $f" >&2; exit 2; }
done

dir=$(dirname "$1")
files=()
for f in "$@"; do
  [[ $f == *.sha256 ]] && continue
  case $(basename "$f") in
  PROVENANCE.txt | LICENSE-DATA) continue ;;
  esac
  files+=("$f")
done
for extra in PROVENANCE.txt LICENSE-DATA; do
  [[ -f $dir/$extra ]] || { echo "missing $dir/$extra" >&2; exit 2; }
  files+=("$dir/$extra")
done

# The index is only publishable if it matches its own manifest. Catch a stale
# or hand-edited artifact before it reaches a release URL.
idx=""
manifest=""
for f in "${files[@]}"; do
  case $f in
  *.idx) idx=$f ;;
  *.idx.json) manifest=$f ;;
  esac
done
if [[ -n $idx && -n $manifest ]]; then
  want=$(jq -r '.index.sha256' "$manifest")
  # The manifest records the body digest, not the whole file.
  have=$(head -c -32 "$idx" | sha256sum | cut -d' ' -f1)
  if [[ $want != "$have" ]]; then
    echo "manifest index.sha256 does not match the index body" >&2
    exit 1
  fi
  echo "index body digest verified against the manifest"
fi

if ! gh release view "$tag" >/dev/null 2>&1; then
  gh release create "$tag" --title "Data $tag" --notes "$(cat <<EOF
OSM-derived geocoding index for Nusapeta Place, release \`$tag\`.

- **Licence**: ODbL 1.0, © OpenStreetMap contributors. See \`LICENSE-DATA\`.
- **Provenance**: source PBF URL and SHA-256, builder version, build command,
  build time, and date are in \`PROVENANCE.txt\`.
- **Integrity**: every artifact has a sibling \`.sha256\`; verify with
  \`sha256sum -c <file>.sha256\`. The manifest's \`index.sha256\` is the digest
  of the index body without its 32-byte trailer, which
  \`nusa-place --verify-checksum\` validates at startup.
- **Format**: PLACEID index v2 (see `docs/INDEX-FORMAT.md`). Readers 0.2.0 and
  later still accept v1 indexes.

Publish a new tag instead of replacing an asset. Readers cache these URLs.
EOF
)"
fi

sums=()
for f in "${files[@]}"; do
  [[ $f == *.sha256 ]] && continue
  # Bare basename so sha256sum -c works after download, when the artifact sits
  # next to its checksum instead of in the build directory.
  (cd "$(dirname "$f")" && sha256sum "$(basename "$f")") > "$f.sha256"
  sums+=("$f.sha256")
done

gh release upload "$tag" "${files[@]}" "${sums[@]}" --clobber
gh release view "$tag" --json assets -q '.assets[].name'
