# Roadmap Nusapeta Place

| Field | Value |
|---|---|
| Status | Draft |
| Tanggal | 2026-11-13 |
| Dokumen terkait | [PRD](PRD.md), [SRS](SRS.md), [TRD](TRD.md), [BACKLOG](BACKLOG.md) |

Roadmap ini mengurutkan pekerjaan berdasarkan risiko, bukan tanggal. Tidak ada
komitmen kalender sampai benchmark pertama selesai. Fase boleh berjalan
paralel hanya jika kontraknya sudah stabil.

## Fase 0: Fondasi dokumen

Status: **sedang dikerjakan**.

Deliverable:

- PRD, SRS, TRD, ARCHITECTURE.
- Kontrak API dan format index.
- Protokol benchmark.
- ADR untuk repo, index, `place_id`, dan API.
- README, lisensi, aturan kontribusi.

Exit criteria:

- Tidak ada requirement `MUST` yang bertentangan antar dokumen.
- Semua tautan internal valid.
- `place_id` dan format index punya spesifikasi yang bisa diimplementasikan.
- Target performa diberi label target, bukan klaim.

## Fase 1: Skeleton dan format

Deliverable:

- Crate Rust dengan dua binary.
- Parser header, section table, places, strings, postings.
- Writer index untuk fixture kecil.
- Manifest dan checksum.
- Validasi index dan test fixture rusak.
- `place_id` encode/decode dan test batas 53-bit.

Exit criteria:

- Fixture PBF kecil bisa dibuild, dibaca ulang, dan divalidasi.
- Index dengan magic, offset, atau checksum salah ditolak.
- Dua build fixture menghasilkan index body identik.
- Layout byte dibekukan untuk `format_version=1`.
- Format v2 menambah section opsional tanpa mengubah layout v1.

## Fase 2: Query minimum

Deliverable:

- Forward search dengan normalisasi dan ranking dasar.
- Reverse dengan grid.
- Lookup `osm_ids` dan `place_ids`.
- `/healthz`, `/version`, `/metrics`.
- Error format dan limit.
- Container image dan smoke test.

Exit criteria:

- Semua endpoint di [API](API.md) merespons fixture.
- Search exact dan prefix berfungsi.
- Reverse mengembalikan fitur terdekat atau 404.
- Lookup mempertahankan urutan permintaan.
- Tidak ada panic pada input tidak valid.

## Fase 3: Build Indonesia

Deliverable:

- Seleksi fitur untuk seluruh PBF Indonesia.
- Resolusi koordinat dan centroid way.
- Polygon admin level 4 sampai 7.
- Name variants dan alias.
- Manifest lengkap dengan jumlah fitur dan peringatan.
- Index Indonesia pertama.

Exit criteria:

- Build selesai dalam target NFR-006 di mesin referensi.
- Index berada di bawah target NFR-003.
- Server start dan melayani query pada index Indonesia.
- Jumlah fitur yang dilewati tercatat dan bisa dijelaskan.
- Tidak ada polygon invalid yang membuat build gagal.

## Fase 4: Kualitas dan rilis kandidat

Deliverable:

- Query set 300 berlabel.
- Evaluasi top-1 dan top-5 per kategori.
- Tuning bobot ranking.
- Perbaikan dedup dan alias.
- Benchmark latency, RSS, startup, dan build.
- Release candidate dengan manifest dan checksum.

Exit criteria:

- Target relevansi di [PRD](PRD.md) tercapai atau deviasi dicatat.
- NFR-001 sampai NFR-012 lulus atau punya rencana perbaikan.
- Hasil benchmark masuk ke [BENCHMARKS](BENCHMARKS.md).
- Rollback diuji.

## Fase 5: Operasi berulang

Deliverable:

- Rebuild bulanan atau sesuai kebutuhan.
- Prosedur promosi dan rollback yang diuji.
- Dashboard dan alert.
- Audit dependency dan lisensi.
- Catatan rilis.

Exit criteria:

- Rebuild kedua berhasil tanpa downtime pada dua replika.
- Rollback ke rilis sebelumnya berhasil.
- Index yang lebih tua dari 45 hari memicu alert.
- Tidak ada data PBF atau index di git.

## Setelah v1

Kandidat, belum dijanjikan:

- Update diff tanpa rebuild penuh.
- Dukungan admin level 8.
- Dedup cabang merek.
- Pencarian kategori tanpa nama.
- Dukungan negara lain.
- Sharding multi-region.
- Output geometri.

Fitur di atas hanya masuk roadmap jika ada kebutuhan pengguna dan hasil
benchmark yang mendukung. Jangan menambah fitur karena "Nominatim punya".

## Definisi selesai

Pekerjaan dianggap selesai jika:

- Perilaku yang diminta ada di kode, bukan hanya dokumen.
- Ada test atau benchmark yang menjalankan perubahan.
- Dokumentasi dan kontrak diperbarui.
- Tidak ada artefak sementara yang tertinggal.
- Risiko baru dicatat di dokumen terkait.
