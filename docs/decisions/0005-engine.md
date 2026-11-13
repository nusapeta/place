# ADR 0005: engine pencarian

| Field | Value |
|---|---|
| Status | Accepted |
| Tanggal | 2026-11-13 |
| Konteks | placeid v1 |

## Konteks

Backlog P1-04 meminta perbandingan custom FST, SQLite FTS5, dan Tantivy pada
korpus nama yang sama. Korpus uji adalah 1.226.648 nama ternormalisasi hasil
`export_names` dari index Indonesia penuh. Query yang dipakai adalah 100 query
berlabel di `docs/research/spikes/queries-100.tsv`; label adalah nama
ternormalisasi yang diharapkan pada posisi teratas.

Angka di bawah adalah **measured** pada host 12 core, 14 GiB RAM, Linux x64,
13 November 2026. SQLite memakai FTS5 `unicode61 remove_diacritics 2` dengan
satu kolom nama. Tantivy 0.27 memakai satu field teks `name` dan satu field
skor `importance`. Keduanya hanya menyimpan nama, tanpa geometri, grid, admin,
atau atribut respons. Custom FST menyimpan seluruh dokumen dan index bantu,
sehingga ukurannya tidak bisa dibandingkan langsung.

| Engine | Build | Ukuran | RSS proses | p95 query | Top-1 50 | Top-1 100 |
|---|---:|---:|---:|---:|---:|---:|
| Custom FST + HTTP | 216,5 s | 141.929.320 B | 6.132 KiB idle | 22,476 ms | 48/50 | 83/100 |
| SQLite FTS5 | 1,95 s | 58.654.720 B | 27.276 KiB | 4,240 ms | 49/50 | 89/100 |
| Tantivy | 1,17 s | 40.512.795 B | 69.924 KiB | 0,259 ms | 41/50 | 80/100 |

Angka 50 query memakai 50 baris pertama `queries-100.tsv`, sesuai acceptance
P1-04. Angka 100 query disertakan agar selisih pada query ambigu tidak
tersembunyi. Setelah perbaikan kandidat dan bobot cakupan token, custom pada
50 query pertama mencapai 48/50 top-1 dan 50/50 top-5 dengan p95 22,476 ms;
angka 100 query pada tabel adalah sebelum perbaikan agar perbandingan engine
memakai perilaku yang sama. Perbaikan dan hasil akhir ada di P4-03.

Catatan pengukuran:

- Build custom adalah wall-clock penuh dari PBF 1,74 GB ke index terverifikasi,
  termasuk tiga pass dan checksum. Build SQLite dan Tantivy hanya dari TSV nama.
- p95 custom diukur lewat HTTP dan mencakup parsing, ranking, dedup, dan JSON.
  p95 SQLite dan Tantivy diukur in-process.
- RSS custom adalah `VmRSS` setelah validasi dan `madvise(MADV_DONTNEED)`, dua
  detik tanpa traffic. RSS SQLite dan Tantivy adalah akhir proses build+query.
- Top-1 custom 48/50 pada 50 query pertama dan 83/100 pada seluruh set sebelum
  perbaikan kandidat P4-03. Dengan `PLACEID_WEIGHT_IMPORTANCE=0`, hasil 100
  query tetap 83/100, jadi bobot importance tidak mengubah peringkat pada set
  itu.
- `top-5` custom 88/100 sebelum perbaikan. Sebagian kegagalan berasal dari
  label yang lebih pendek daripada nama kanonik, misalnya `bandung` versus
  `kota bandung`; setelah label menerima nama kanonik, hasilnya naik.

## Keputusan

v1 memakai custom mmap index sesuai ADR 0002. SQLite FTS5 dan Tantivy tidak
dipakai sebagai engine utama.

Alasan yang bisa diuji:

1. Hanya custom yang menyimpan satu file read-only berisi places, strings,
   varian, postings, name FST, token FST, grid, dan admin. SQLite dan Tantivy
   membutuhkan skema tambahan atau file terpisah untuk mencapai perilaku yang
   sama.
2. RSS idle custom 6,1 MB, di bawah NFR-004 50 MB. RSS SQLite 27,3 MB dan
   Tantivy 69,9 MB pada pengukuran yang tidak memuat seluruh atribut.
3. Startup custom 237 ms sampai `/healthz` pada index penuh, di bawah NFR-005
   1 detik, setelah validasi penuh dan pelepasan halaman.
4. Top-1 SQLite lebih tinggi 6 poin pada korpus nama saja, tetapi belum
   mengukur ranking multi-token, dedup, jarak, dan atribusi yang menjadi
   tanggung jawab custom. Selisih ini dicatat sebagai risiko relevansi, bukan
   alasan tersembunyi untuk menolak migrasi.

Pemicu migrasi ke SQLite FTS5, dievaluasi di P4-03 dan P4-06:

- Top-1 di bawah 80% atau top-5 di bawah 95% pada set holdout setelah tuning.
- p95 `/search` di atas NFR-001 pada beban target.
- Kebutuhan update inkremental yang tidak bisa dipenuhi rebuild penuh.

Jika salah satu pemicu terjadi, SQLite FTS5 menjadi kandidat pertama karena
top-1 dan latensinya lebih baik pada korpus yang sama.

## Konsekuensi

Positif:

- Satu file index, tanpa service tambahan, dan tanpa write path saat serve.
- RSS idle rendah karena halaman bersih dilepas setelah validasi.
- Format section dapat diperluas tanpa migrasi database.

Negatif:

- Build 216,5 s, jauh lebih lambat daripada membangun index nama SQLite.
- Relevansi awal lebih rendah daripada SQLite pada korpus nama saja.
- Decoder, validasi, dan tooling index harus dipelihara sendiri.
- Page fault saat query pertama setelah startup menambah latensi ekor.

## Alternatif yang ditolak

| Alternatif | Alasan |
|---|---|
| SQLite FTS5 | Top-1 lebih baik, tetapi butuh skema dan file tambahan untuk atribut, geometri, dan ranking; RSS lebih tinggi; belum mengukur seluruh perilaku. |
| Tantivy | Latensi terbaik, tetapi top-1 terendah (80/100), RSS tertinggi (69,9 MB), dan menambah runtime pencarian besar. |
| Hybrid nama SQLite + metadata custom | Dua sumber kebenaran, dua proses validasi, dan checksum terpisah; biaya operasional tidak sepadan untuk v1. |

## Tinjau ulang

Tinjau ulang setelah P4-03 dan P4-06. Jika pemicu migrasi tercapai, buat ADR
baru yang mencabut ADR ini, pertahankan `place_id` dan kontrak API, lalu
implementasikan engine SQLite di belakang trait yang sama.
