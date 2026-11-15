# Quickstart

## From the release artifact

Requires Docker, curl, and sha256sum.

```bash
mkdir -p place-data && cd place-data
base=https://github.com/nusapeta/place/releases/download/data-2026.11

curl -fsSLO "$base/places-2026.11.idx"
curl -fsSLO "$base/places-2026.11.idx.json"
curl -fsSLO "$base/places-2026.11.idx.sha256"
sha256sum -c places-2026.11.idx.sha256

docker run --rm -p 127.0.0.1:8090:8090 -v "$PWD:/data:ro" \
  ghcr.io/nusapeta/place:0.1.0 \
  --index /data/places-2026.11.idx \
  --manifest /data/places-2026.11.idx.json \
  --verify-checksum
```

Query it:

```bash
curl -s 'http://127.0.0.1:8090/search?q=monas&limit=3' | jq
curl -s 'http://127.0.0.1:8090/reverse?lat=-6.175392&lon=106.827153' | jq
curl -s 'http://127.0.0.1:8090/healthz' | jq
```

The release checksum covers the whole file. The manifest's `index.sha256`
covers the index body without its 32-byte trailer; `--verify-checksum` validates
the trailer. Do not compare the manifest digest with `sha256sum` output. See
[`../releases/0.1.0.md`](../releases/0.1.0.md).

The index is 142 MB. The container runs as UID 65532 with the index volume
read-only.

## Combined stack

```bash
git clone https://github.com/nusapeta/stack.git
cd stack
cp .env.example .env
scripts/bootstrap.sh
docker compose up -d --wait
scripts/e2e.sh
```

The default stack serves place search through the gateway at
`http://127.0.0.1:8080/place/search?q=...`. The stack quickstart is
[`https://github.com/nusapeta/stack/blob/main/docs/QUICKSTART.md`](https://github.com/nusapeta/stack/blob/main/docs/QUICKSTART.md).

## Build from a PBF

Requires Rust 1.99.1 (see `rust-toolchain.toml`) and a national OSM PBF.

```bash
git clone https://github.com/nusapeta/place.git
cd place
cargo build --release --bins

mkdir -p /srv/nusapeta-place/releases/2026-11
/usr/bin/time -v target/release/nusa-place-build \
  --input /data/indonesia-latest.osm.pbf \
  --output /srv/nusapeta-place/releases/2026-11/places.idx \
  --manifest /srv/nusapeta-place/releases/2026-11/places.idx.json \
  --verify
```

A national build takes 228 to 288 s and about 8 GiB peak RAM on the test
machine (228.42 s for the benchmark run recorded in `docs/BENCHMARKS.md`, 288 s
for the build behind the shipped `data-2026.11` index). The builder writes a
temporary file and renames it atomically; a failed build leaves the old index
untouched.

Check the result:

```bash
jq '.format_version, .counts, .index.sha256' \
  /srv/nusapeta-place/releases/2026-11/places.idx.json
```

## Run and verify

```bash
target/release/nusa-place \
  --index /srv/nusapeta-place/releases/2026-11/places.idx \
  --manifest /srv/nusapeta-place/releases/2026-11/places.idx.json \
  --verify-checksum \
  --bind 127.0.0.1:18090

curl -fsS http://127.0.0.1:18090/healthz
curl -fsS http://127.0.0.1:18090/version
```

`/healthz` returns `format_version: 1`, not the route graph's format 5. Stop the
temporary server after both endpoints return 200.

## Local fixture

The repo includes a small PBF fixture for tests; it is not national data.

```bash
scripts/make-fixtures.sh
cargo test
```

The fixture covers a handful of features and is only for builder/reader tests.
Do not benchmark relevance on it.

## Verify a checkout

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-targets
cargo deny check
```

Full runbook: [`../OPERATIONS.md`](../OPERATIONS.md). API contract:
[`API.md`](API.md).
