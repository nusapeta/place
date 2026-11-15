# ADR 0007: sumber importance

| Field | Value |
|---|---|
| Status | Accepted |
| Tanggal | 2026-11-13 |
| Konteks | Nusapeta Place v1 |

## Konteks

FR-006 meminta `importance` 0 sampai 1 untuk ranking. P1-09 membandingkan
sumber tag OSM saja dengan tabel Wikipedia/Wikidata. Set uji adalah 100 query
berlabel `docs/research/spikes/queries-100.tsv` pada index Indonesia penuh.

**Measured** pada 13 November 2026:

| Konfigurasi | Top-1 | Top-5 | p95 |
|---|---:|---:|---:|
| Bobot importance 0,5 (default) | 83/100 | 88/100 | 31,415 ms |
| Bobot importance 0 (dinonaktifkan) | 83/100 | 88/100 | 31,845 ms |

Kedua konfigurasi identik pada set ini. Artinya sinyal importance yang ada
belum mengubah urutan top-1 atau top-5 untuk query yang diuji. Efeknya baru
terlihat pada tie-break ketika skor teks dan jarak sama.

Distribusi importance index penuh: minimum 0,240 dan maksimum 0,990. Formula
memakai kelas OSM, bonus `population`, `capital`, `wikidata`, dan `brand`.
Tidak ada tabel eksternal yang dibangun, sehingga biaya ukuran tabel eksternal
belum diukur dan tidak diklaim.

## Keputusan

v1 memakai tag OSM saja. Tidak ada tabel Wikipedia atau Wikidata pada build
maupun index.

Alasan:

1. Tidak ada kenaikan top-1 atau top-5 yang terukur dari menambah bobot
   importance pada set 100 query.
2. Tabel eksternal menambah sumber data, lisensi, jadwal update, dan
   rekonsiliasi ID. Wikidata berlisensi CC0, tetapi Wikipedia memakai CC BY-SA
   dan membutuhkan atribusi tambahan pada output.
3. Setiap baris eksternal menambah ukuran index dan waktu build tanpa bukti
   relevansi pada data ini.
4. Tag OSM sudah menyediakan `population`, `capital`, `wikidata`, dan `brand`
   yang cukup untuk tie-break.

Keputusan ini tidak menutup penambahan tabel eksternal di v2. Jika dilakukan,
ADR baru **MUST** mencatat ukuran terukur, lisensi, atribusi, dan hasil
perbandingan top-1 pada set holdout.

## Konsekuensi

Positif:

- Tidak ada lisensi atau atribusi baru di luar ODbL.
- Build tetap satu sumber, PBF.
- Tidak ada kunci rekonsiliasi yang bisa salah.

Negatif:

- Importance tidak menangkap popularitas yang tidak tercatat di OSM.
- Fitur dengan tag sedikit bisa memiliki skor rendah meskipun dikenal publik.
- Query yang hanya dibedakan oleh importance tetap bergantung pada tie-break
  lain.

## Alternatif yang ditolak

| Alternatif | Alasan |
|---|---|
| Tabel Wikidata per `wikidata` ID | Belum ada bukti kenaikan top-1; menambah ukuran, lisensi CC0, dan jadwal update. |
| Tabel Wikipedia pageviews | Data berubah cepat, butuh pipeline eksternal, dan lisensi serta atribusi bertambah. |
| Bobot importance lebih tinggi tanpa data | Tidak terukur; dapat menaikkan fitur administratif dan menggeser hasil yang benar. |
| Importance dari jumlah tag | Proxy mudah dimanipulasi dan tidak berkorelasi langsung dengan kepentingan pengguna. |

## Tinjau ulang

Tinjau ulang jika P4-03 gagal mencapai top-1 80% atau top-5 95% dan analisis
kesalahan menunjukkan importance sebagai penyebab. Ukur tabel eksternal pada
set tune, lalu validasi pada holdout sebelum diadopsi.
