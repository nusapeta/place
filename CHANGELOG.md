# Changelog

All user-visible changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[SemVer](https://semver.org/). The index format has its own version.

## [Unreleased]

## [0.2.1] - 2026-11-20

### Fixed

- `GET /suggest` and `GET /admin/reverse` are registered at the paths the API
  documents. 0.2.0 mounted them as `/place/suggest` and `/place/admin/reverse`,
  which is the prefix the bundle adds in front of the geocoder, so both answered
  404 once the gateway stripped `/place` before forwarding.

### Changed

- The server prints the address it actually bound, so `--bind 127.0.0.1:0` is
  usable by a caller that needs the kernel-assigned port.

## [0.2.0] - 2026-11-20

Binary and data release. Index format v2 requires a rebuild; v1 indexes still
load when the new sections are flagged absent.

### Added

- `GET /suggest` prefix walk over the existing name and token FSTs (NUSA-9):
  minimum 3 characters, capped at 2,000 rows, ranked by match kind, importance,
  and optional grid proximity bias, returned as jsonv2 with `match` and
  `score`.
- Reverse geocoding now measures distance to stored way polylines for long
  roads and returns `distance_m` (NUSA-1).
- `GET /admin/reverse` returns the deepest admin area from polygon data,
  with hierarchy, `ambiguous`, and 501 `no_admin_polygons` (NUSA-1).

### Changed

- Index format bumped to v2 with optional admin polygon and way-line sections;
  readers still accept v1 when flags are zero (NUSA-1). The national index grows
  from 141,997,032 B to 386,376,512 B; postings grow from 4,837,518 to
  29,110,855 rows because grid cells now index the whole feature bbox instead
  of the centroid cell.
- Idle RSS on the v2 national index is 6.3 MiB (`RssAnon` 0.7 MiB), measured
  2026-11-20; startup still mmaps the index and does not preload it.

## [0.1.0] - 2026-11-15

First public release. Project name: Nusapeta Place (previously placeid).

### Added

- Indonesian PBF builder into an mmap index, format v1, with checksum
  verification.
- Read-only HTTP server: `/search`, `/reverse`, `/lookup`, `/healthz`,
  `/version`, `/metrics`.
- Full index 141,997,032 B from a 1,741,650,872 B PBF, 1,226,755 features.
- Reverse centroid with a default 1000 m radius.
- Debian bookworm-slim container, UID 65532, read-only, 256 MB memory limit
  (512 MB in the initial deployment).
- Prometheus metrics, alert rules, dashboard, and promote/rollback scripts.
- amd64 and arm64 multi-architecture builds.

### Security

- `cargo audit` accepts `RUSTSEC-2026-0186` (`memmap2` 0.5.10 through
  `osmpbf` 0.3.8) because only `Mmap::map` is used; `advise_range`,
  `flush_range`, and `flush_async_range` are not called. The ignore is recorded
  in `.cargo/audit.toml` and removed once `osmpbf` uses `memmap2 >= 0.9.11`.

### Known limitations

- Indonesia only; no cross-country search.
- Index format is not stable; the crate sets `publish = false`.
- No built-in authentication, TLS, or rate limit. Run it behind a proxy.

[Unreleased]: https://github.com/nusapeta/place/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/nusapeta/place/compare/v0.1.0...v0.2.0
[0.1.0]: https://github.com/nusapeta/place/releases/tag/v0.1.0
