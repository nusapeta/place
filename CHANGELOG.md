# Changelog

Semua perubahan yang terlihat pengguna dicatat di sini. Format mengikuti
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versi mengikuti
[SemVer](https://semver.org/). Format index punya versi sendiri.

## [Unreleased]

## [0.1.0] - 2026-11-13

Rilis publik pertama. Nama proyek: Nusapeta Place (sebelumnya placeid).

### Added

- Builder PBF Indonesia ke index mmap format v1, dengan verifikasi checksum.
- Server HTTP read-only: `/search`, `/reverse`, `/lookup`, `/healthz`,
  `/version`, `/metrics`.
- Index penuh 141.997.032 B dari PBF 1.741.650.872 B, 1.226.755 fitur.
- Reverse centroid dengan radius default 1000 m.
- Container Debian bookworm-slim, UID 65532, read-only, batas memori 256 MB
  (512 MB pada deployment awal).
- Metrik Prometheus, alert rules, dashboard, dan skrip promote/rollback.
- Build multi-arsitektur amd64 dan arm64.

### Security

- `cargo audit` menerima `RUSTSEC-2026-0186` (`memmap2` 0.5.10 lewat
  `osmpbf` 0.3.8) karena hanya `Mmap::map` yang dipakai; `advise_range`,
  `flush_range`, dan `flush_async_range` tidak dipanggil. Ignore dicatat di
  `.cargo/audit.toml` dan dicabut setelah `osmpbf` memakai `memmap2 >= 0.9.11`.

### Known limitations

- Cakupan Indonesia saja; tidak ada pencarian lintas negara.
- Format index belum stabil; `publish = false` di crate.
- Tanpa autentikasi, TLS, atau rate limit bawaan. Jalankan di belakang proxy.

[Unreleased]: https://github.com/nusapeta/place/compare/v0.1.0...HEAD
[0.1.0]: https://github.com/nusapeta/place/releases/tag/v0.1.0
