#!/usr/bin/env bash
# Rebuild a small place index fixture for nusapeta/stack from a PBF.
#   scripts/make-fixtures.sh <mini.osm.pbf> <out.idx> <out.idx.json>
set -euo pipefail
pbf=${1:?usage: $0 <mini.osm.pbf> <out.idx> <out.idx.json>}
out=${2:?usage: $0 <mini.osm.pbf> <out.idx> <out.idx.json>}
manifest=${3:?usage: $0 <mini.osm.pbf> <out.idx> <out.idx.json>}
cd "$(dirname "$0")/.."
cargo run --release --bin nusa-place-build -- \
  --input "$pbf" --output "$out" --manifest "$manifest" --verify
