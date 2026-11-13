# Dependency, lisensi, dan atribusi

Dihasilkan dari `Cargo.lock` melalui `cargo metadata --format-version 1
--locked` pada 2026-11-13. Jumlah paket: 222. Semua lisensi yang ditemukan
adalah lisensi permisif; tidak ada GPL, AGPL, atau lisensi proprietary yang
diwajibkan.

| Lisensi | Jumlah paket |
|---|---:|
| MIT OR Apache-2.0 | 125 |
| MIT | 45 |
| Apache-2.0 OR MIT | 8 |
| Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | 7 |
| MIT/Apache-2.0 | 5 |
| Unlicense OR MIT | 3 |
| Zlib OR Apache-2.0 OR MIT | 3 |
| Apache-2.0 OR ISC OR MIT | 3 |
| Unlicense/MIT | 2 |
| MIT OR Apache-2.0 OR LGPL-2.1-or-later | 2 |
| ISC | 2 |
| Apache-2.0 | 2 |
| (MIT OR Apache-2.0) AND Unicode-3.0 | 2 |
| BSD-2-Clause OR Apache-2.0 OR MIT | 2 |
| Lisensi permisif lainnya | 31 |

Dua paket `r-efi` 5.3.0 dan 6.0.0 menawarkan pilihan
`MIT OR Apache-2.0 OR LGPL-2.1-or-later`. Kami memilih MIT atau Apache-2.0,
sehingga kewajiban LGPL tidak berlaku. Paket tersebut hanya masuk target
UEFI dan tidak dipakai pada build Linux.

## Kewajiban atribusi

- Data OpenStreetMap berlisensi ODbL 1.0. Setiap respons API memuat field
  `licence` berisi `ODbL 1.0, © OpenStreetMap contributors`, diuji oleh test
  kontrak.
- Distribusi index atau turunan data OSM harus menyertakan atribusi dan
  menawarkan database turunan di bawah ODbL. Index `placeid` adalah database
  turunan ODbL.
- Kode `placeid` berlisensi MIT; file `LICENSE` disertakan di repo.
- PBF dari Geofabrik tidak disimpan di repo. Pengguna harus mengunduh sendiri
  dan mematuhi ketentuan Geofabrik serta OSM.

## Audit keamanan

`cargo audit 0.23.0` dijalankan pada 2026-11-13 dengan lockfile yang sama:

- 0 vulnerability.
- 1 warning `RUSTSEC-2026-0186`: `memmap2` 0.5.10, ditarik oleh `osmpbf`
  0.3.8, memiliki validasi kurang pada `advise_range` dan `flush_range`.
  `placeid` tidak memanggil API tersebut; `release_resident_pages` memakai
  `libc::madvise` langsung pada slice yang sudah divalidasi. `osmpbf` 0.3.8
  adalah versi terbaru dan belum memakai `memmap2` 0.9. Warning ini diterima
  dengan alasan jalur kode tidak terpanggil dan dipantau saat update
  dependency.

## Batas

Daftar ini adalah snapshot. Dependency baru atau update lockfile wajib
menjalankan ulang `cargo metadata`, `cargo audit`, dan pemeriksaan lisensi
sebelum release.
