# ADR 0001: Repo terpisah dan service Rust

| Field | Value |
|---|---|
| Status | Accepted |
| Tanggal | 2026-11-13 |
| Konteks | Nusapeta Place v1 |

## Konteks

routeid adalah router Go yang sudah berjalan dan punya siklus rilis sendiri.
Geocoder punya lifecycle berbeda: build batch dari PBF, artefak index besar,
benchmark RSS, dan format file sendiri. Menaruh keduanya di satu repo berarti
CI, dependency, dan rilis saling mengunci.

## Keputusan

- `nusapeta-place` berada di repo terpisah.
- Implementasi memakai Rust, bukan Go.
- Integrasi dengan routeid lewat HTTP, bukan FFI atau cgo.
- Tidak ada kode yang dibagi antar repo. Kontraknya adalah API dan format
  manifest.
- Gateway routeid yang menangani auth, TLS, dan rate limit.

## Konsekuensi

Positif:

- Siklus rilis dan CI terpisah.
- Rust memberi kontrol atas mmap, alokasi, dan RSS.
- Panic atau bug geocoder tidak menjatuhkan router.
- Index bisa dipromosikan tanpa rebuild router.

Negatif:

- Dua toolchain dan dua pipeline rilis.
- Perlu kontrak HTTP yang diuji di kedua sisi.
- Tidak ada tipe bersama; perubahan API harus didokumentasikan.

## Alternatif yang ditolak

| Alternatif | Alasan |
|---|---|
| Monorepo Go | Lifecycle dan dependency bercampur, index besar mengganggu CI router |
| Library Go dipanggil via cgo | Batas allocator dan panic, build lintas platform lebih sulit |
| Nominatim eksternal | Service database berat, tidak sesuai target footprint |
| FFI Rust ke Go | Kompleksitas build dan debugging tidak sepadan |

## Tinjau ulang

Jika routeid dan Nusapeta Place akhirnya selalu dirilis bersamaan dan kontrak HTTP
menjadi beban, monorepo bisa dipertimbangkan. Sampai itu terjadi, repo
terpisah adalah default.
