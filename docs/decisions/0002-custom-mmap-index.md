# ADR 0002: Index mmap khusus

| Field | Value |
|---|---|
| Status | Accepted untuk v1 |
| Tanggal | 2026-11-13 |
| Konteks | placeid v1 |

## Konteks

Target produk menuntut satu file index, RSS idle kecil, startup cepat, dan
tanpa service database. Ada beberapa cara membangun pencarian:

- SQLite FTS5 dengan tabel spasial.
- Tantivy.
- PostgreSQL + PostGIS + Nominatim.
- Index custom yang dipetakan dengan `mmap`.

Korpus Indonesia yang diukur berisi 1.263.949 fitur bernama dan 21,2 MB teks.
Data ini cukup kecil untuk index satu file, tetapi cukup besar untuk membuat
pencarian linear per request tidak masuk akal.

## Keputusan

Bangun index custom read-only dengan layout di
[INDEX-FORMAT](../INDEX-FORMAT.md). Format memuat record fixed-size, string
blob, token FST, postings, name FST, grid spasial, dan admin record.

SQLite FTS5 tetap menjadi fallback yang serius. Jika benchmark menunjukkan
SQLite memenuhi target dengan biaya perawatan lebih rendah, keputusan ini
harus ditinjau ulang. Membangun format custom bukan tujuan produk.

## Konsekuensi

Positif:

- Satu file, tanpa service database.
- Kontrol penuh atas ukuran record, postings, dan urutan pembacaan.
- Startup hanya memvalidasi header dan memetakan file.
- Format bisa di-fuzz dan divalidasi ketat.

Negatif:

- Format dan parser harus dirawat sendiri.
- Tidak ada query planner atau indeks sekunder gratis.
- Perubahan layout memerlukan versioning dan test kompatibilitas.
- Ranking dan dedup harus ditulis sendiri.

## Alternatif yang ditolak

| Alternatif | Alasan |
|---|---|
| SQLite FTS5 | C dependency, query planner di luar kontrol, tetap fallback |
| Tantivy | Multi-file, fokus full-text umum, tidak punya grid/admin |
| PostgreSQL + PostGIS | Service database, RAM besar, impor panjang |
| Index in-memory | Startup dan RSS tidak sesuai target |
| Pencarian linear | Tidak memenuhi target latency untuk 1,26 juta fitur |

## Gate

Keputusan dipertahankan hanya jika:

- Index Indonesia ≤ 600 MB.
- Idle RSS ≤ 50 MB.
- Search p95 ≤ 50 ms pada 20 RPS.
- Format parser lulus fuzz dan test fixture rusak.

Jika salah satu gate gagal dan SQLite FTS5 bisa memenuhinya, migrasi ke SQLite
lebih baik daripada memaksakan format custom.
