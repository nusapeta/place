# Changelog

All user-visible changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[SemVer](https://semver.org/). The index format has its own version.

## [Unreleased]

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

[Unreleased]: https://github.com/nusapeta/place/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/nusapeta/place/releases/tag/v0.1.0
