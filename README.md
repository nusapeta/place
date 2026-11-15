# Nusapeta Place

A self-hosted OpenStreetMap geocoder for Indonesia. One Rust crate, two
binaries (batch builder and HTTP server), one memory-mapped index, no database
service.

[![Rust](https://img.shields.io/badge/Rust-1.99-000000?logo=rust)](rust-toolchain.toml)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)
[![Data](https://img.shields.io/badge/data-ODbL-green)](https://www.openstreetmap.org/copyright)
[![Status](https://img.shields.io/badge/status-research%20preview-orange)](#limitations)

[Bahasa Indonesia](README.id.md) | English

**A self-hosted geo stack for Indonesia: routing + geocoding, one file per
engine, under 20 MB idle, rebuilt in minutes, no database.** This repository is
the geocoding half; the router is
[`nusapeta/route`](https://github.com/nusapeta/route) and the compose bundle is
[`nusapeta/stack`](https://github.com/nusapeta/stack).

`6.15 MiB` idle RSS · `142 MB` index · `228 to 288 s` to build from the
national PBF · `±17 MB` for the route + place stack when idle.

**Not Google Place ID.** Nusapeta Place has no affiliation with Google. Its
`place_id` is a stable 53-bit identifier derived from `(osm_type, osm_id)`, not
a Google Place ID and not interchangeable with one.

## Quick start from the release artifact

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

In another terminal:

```bash
curl 'http://127.0.0.1:8090/search?q=monas&limit=3'
curl 'http://127.0.0.1:8090/reverse?lat=-6.175392&lon=106.827153'
curl 'http://127.0.0.1:8090/healthz'
```

The index checksum has two forms. The release `.sha256` covers the whole file
(`a732b4ec…836d`). The manifest's `index.sha256` covers the index body without
its 32-byte trailer (`0df44207…2bca`); `--verify-checksum` checks that trailer.
See [`docs/releases/0.1.0.md`](docs/releases/0.1.0.md).

**Read more:** [what it solves](#what-it-solves) · [measured
corpus](#measured-corpus) · [scope](#scope-v1) · [technical
shape](#technical-shape) · [limits](#limitations) · [build from
source](#build-from-source) · [documentation](#documentation)

## What it solves

Apps that need Indonesian place search usually choose a hosted API with
per-request pricing, or run full Nominatim with PostgreSQL, several GB of RAM,
and a long import. For a narrow workload (place names, POIs, roads, and nearest
feature), most of that capacity is unused.

Nusapeta Place takes a smaller problem boundary:

- Indonesia only in v1.
- Place names, POIs, roads, and administrative areas.
- Forward search, reverse nearest, and ID lookup.
- Full rebuild from a PBF, not diff updates.
- One index file that can be promoted and rolled back as an artifact.

Measured footprint and reproduction steps: [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md).
The route engine adds about 11 MB idle RSS, so the combined stack idles near
17 MB.

Research preview, one maintainer, no SLA. Data artifacts are refreshed monthly
on a best-effort basis. Large feature pull requests MAY be declined even when
correct; open an issue before writing one.

The combined routing and geocoding stack is
[`nusapeta/stack`](https://github.com/nusapeta/stack).

## Build from source

The toolchain is pinned in `rust-toolchain.toml`: Rust 1.99.1 with `rustfmt` and
`clippy`, minimal profile.

```bash
git clone https://github.com/nusapeta/place.git
cd place

cargo build --release --bins

target/release/nusa-place-build \
  --input data/indonesia-latest.osm.pbf \
  --output /data/places.idx \
  --manifest /data/places.idx.json \
  --verify

target/release/nusa-place \
  --index /data/places.idx \
  --manifest /data/places.idx.json \
  --bind 127.0.0.1:8090
```

A full national build needs about 8 GiB peak RAM. The final index is
141,997,032 B for 1,226,755 features and 9,360 administrative areas. The
container listens on 8090 and expects the index under `/data`.

## Measured corpus

The census was run on 2026-11-13 against `data/indonesia-latest.osm.pbf`. These
are corpus counts, not index results.

| Object | Total | Named | With `addr:housenumber` |
|---|---:|---:|---:|
| Nodes | 277,198,053 | 346,846 | 39,386 |
| Ways | 50,921,496 | 878,076 | 67,702 |
| Relations | 71,229 | 39,027 | 0 |
| Total | 328,190,778 | 1,263,949 | 107,088 |

There are 645,238 raw unique names, 21,243,045 bytes of name text, and 612,877
names after normalization. Name text is not the main cost: records, postings,
the spatial grid, and optional administrative polygons dominate. House-number
geocoding is out of scope for v1: only 107,088 objects have
`addr:housenumber`, unevenly distributed.

## Scope v1

- `GET /search?q=...` for place names, POIs, roads, and areas.
- `GET /reverse?lat=...&lon=...` for the nearest named feature.
- `GET /lookup?osm_ids=...` and `place_id` lookup.
- Administrative hierarchy for `display_name` and `address` when polygons are
  available.
- Indonesian name normalization: lowercase, diacritics, common abbreviations
  such as `Jl.`, `Kec.`, `Kab.`, `Kel.`, `Kp.`.
- Aliases from `name:id`, `name:en`, `alt_name`, `old_name`, `official_name`,
  `short_name`, and `brand`.
- Operational endpoints: `/healthz`, `/version`, `/metrics`.

## Non-goals v1

- Full Nominatim parity.
- House-number geocoding and address interpolation.
- Daily diff updates or streaming replication.
- Routing, map matching, isochrones, or polygon output.
- Worldwide coverage. The v1 index is built for Indonesia.
- A multi-tenant public service with billing and an SLA.

## Technical shape

```text
Indonesia PBF
    |
    v
nusa-place-build  (Rust, batch)
    |
    v
places.idx + places.idx.json
    |
    v
nusa-place  (Rust, HTTP, mmap read-only)
```

The index stores feature records, original strings, token FSTs, postings, a
name FST, a spatial grid, and optional administrative polygons. `place_id` is
derived from `(osm_type, osm_id)` and fits in the 53-bit safe-integer range for
JavaScript clients. Contract details:
[`docs/INDEX-FORMAT.md`](docs/INDEX-FORMAT.md).

## Comparison, with limits

| Property | Nusapeta Place | Nominatim |
|---|---|---|
| Storage engine | custom mmap index | PostgreSQL + PostGIS |
| Index / database size | 142 MB | not measured here |
| Idle RSS | 6.15 MiB | not measured here |
| House-number geocoding | no | yes |
| Update model | full rebuild | diff replication |
| Coverage | Indonesia | worldwide |

No head-to-head latency or relevance benchmark against Nominatim is claimed.
The place benchmark in [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md) covers this
engine only.

## Limitations

- **No house-number accuracy claim.** Address coverage is sparse.
- **No real-time updates.** A new index is a full rebuild and a restart.
- **Relevance is measured, not guaranteed.** The benchmark reports this corpus;
  it is not a promise for every query.
- **Administrative polygons are incomplete.** The national build logged 46
  missing refs and 43 invalid polygons; affected features get a shorter
  `display_name`.
- **Not Google Place ID.** See the disclaimer at the top.

## Documentation

| Document | Content |
|---|---|
| [`docs/en/QUICKSTART.md`](docs/en/QUICKSTART.md) | release artifact to first query |
| [`docs/en/API.md`](docs/en/API.md) | English HTTP API contract |
| [`docs/en/ARCHITECTURE.md`](docs/en/ARCHITECTURE.md) | English architecture copy |
| [`docs/API.md`](docs/API.md) | canonical HTTP contract and Nominatim subset |
| [`docs/ARCHITECTURE.md`](docs/ARCHITECTURE.md) | context, containers, build and query flow |
| [`docs/INDEX-FORMAT.md`](docs/INDEX-FORMAT.md) | index layout, validation, versioning |
| [`docs/BENCHMARKS.md`](docs/BENCHMARKS.md) | benchmark protocol and results |
| [`docs/OPERATIONS.md`](docs/OPERATIONS.md) | build, deploy, rebuild, rollback |
| [`docs/ROADMAP.md`](docs/ROADMAP.md) | phases and exit criteria |
| [`docs/research/`](docs/research/) | method research and evidence |
| [`deploy/README.md`](deploy/README.md) | container, Compose, tunnel, rollback |
| [`docs/releases/0.1.0.md`](docs/releases/0.1.0.md) | release notes |

The document index and change rules are in [`docs/README.md`](docs/README.md).

## License

Code: MIT, see [LICENSE](LICENSE). OpenStreetMap data: ODbL 1.0, see
[`LICENSE-DATA`](LICENSE-DATA). API responses and documentation MUST carry
attribution to OpenStreetMap contributors.
