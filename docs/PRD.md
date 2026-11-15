# PRD: Nusapeta Place

| Field | Value |
|---|---|
| Status | Draft |
| Versi | 0.1 |
| Tanggal | 2026-11-13 |
| Pemilik | arifsans |
| Dokumen terkait | [SRS](SRS.md), [TRD](TRD.md), [ROADMAP](ROADMAP.md) |

## 1. Ringkasan

`nusapeta-place` adalah layanan geocoding mandiri untuk Indonesia yang dibangun dari
OpenStreetMap. Layanan ini menjawab tiga kebutuhan: mencari tempat berdasarkan
nama, menemukan fitur bernama di sekitar koordinat, dan membuka kembali tempat
dari identifier yang stabil.

Bentuk yang dituju: satu crate Rust dengan dua binary, builder batch dan
server HTTP, satu file index `mmap`, tanpa PostgreSQL. Index dibangun ulang
dari file PBF dan dipromosikan sebagai artefak. Layanan berjalan di belakang
gateway routeid atau langsung di jaringan internal.

## 2. Masalah

Ada tiga pilihan yang ada sekarang, dan semuanya punya biaya:

| Pilihan | Masalah untuk kasus ini |
|---|---|
| Hosted geocoding API | Biaya per request, kuota, dan ketergantungan jaringan |
| Nominatim lengkap | PostgreSQL, impor panjang, RAM beberapa GB, dan banyak fitur yang tidak dipakai |
| Mencari sendiri di tabel OSM | Normalisasi nama, ranking, alias, dan admin hierarchy dikerjakan ulang di setiap aplikasi |

Kebutuhan sebenarnya sempit. Aplikasi ingin mengetik nama tempat, mendapat
koordinat dan nama tampilan, lalu mungkin mencari nama jalan dari titik GPS.
Untuk itu, tidak perlu parity Nominatim, tidak perlu dukungan dunia, dan tidak
perlu geocoding nomor rumah.

Data OSM Indonesia yang diukur pada 2026-11-13 berisi 1.263.949 fitur bernama,
645.238 nama unik, dan 21,2 MB teks nama. Hanya 107.088 objek yang punya
`addr:housenumber`. Korpus ini cukup untuk pencarian nama, tetapi tidak cukup
untuk mengklaim akurasi alamat.

## 3. Pengguna

| Pengguna | Kebutuhan |
|---|---|
| Operator routeid | Proxy pencarian tempat dari gateway tanpa menambah database baru |
| Pengembang aplikasi internal | Jalankan satu container dan satu file index di VPS kecil |
| Engineer data | Rebuild index bulanan dari PBF, verifikasi asal data, dan rollback |
| Maintainer | Format index dan ranking yang bisa diuji, bukan heuristic yang tidak terlihat |

## 4. Jobs to be done

- Saat pengguna mengetik nama tempat, tampilkan kandidat yang relevan dalam
  waktu yang tidak terasa lambat.
- Saat pengguna membuka peta di suatu titik, tampilkan nama jalan, kelurahan,
  kecamatan, atau tempat terdekat.
- Saat aplikasi menyimpan `place_id`, buka kembali tempat yang sama setelah
  index dibangun ulang.
- Saat data OSM berubah, ganti index tanpa mengubah kontrak API.
- Saat hasil terlihat salah, telusuri apakah masalahnya ada di data, ranking,
  atau format index.

## 5. Tujuan produk

1. Pencarian nama tempat, POI, jalan, dan wilayah administratif Indonesia
   dengan hasil yang bisa dijelaskan.
2. Reverse geocoding ke fitur bernama terdekat, bukan interpolasi alamat.
3. Identifier tempat yang stabil antar rebuild selama objek OSM masih ada.
4. Footprint operasional kecil: satu proses, satu file index, tanpa database.
5. Build yang deterministik dari PBF dan versi builder yang sama.
6. Kontrak HTTP yang cukup mirip Nominatim `jsonv2` untuk memindahkan klien
   dengan perubahan kecil.
7. Prosedur rebuild dan rollback yang bisa dijalankan tanpa menebak.

## 6. Non-goals

- Parity penuh Nominatim, termasuk semua parameter dan format output.
- Geocoding nomor rumah, interpolasi alamat, atau pencarian "Jl. X No. 12".
- Update diff, streaming replication, atau konsistensi real-time dengan OSM.
- Routing, map matching, isochrone, atau navigasi.
- Polygon output, `polygon_geojson`, dan geometri jalan lengkap.
- Cakupan dunia. Index v1 hanya untuk Indonesia.
- Layanan publik dengan billing, API key, dan SLA multi-tenant.
- Pencarian natural language seperti "warung dekat sini yang buka malam".

## 7. Scope v1

### 7.1 Masuk

- Forward search berbasis nama dan alias.
- Reverse nearest untuk fitur bernama.
- Lookup berdasarkan `place_id` dan `osm_ids`.
- Hierarki administratif untuk `display_name` dan `address` bila polygon
  tersedia.
- Normalisasi dan singkatan nama Indonesia.
- Filter bahasa sederhana: `name:id` dan `name:en`.
- Endpoint health, version, dan metrics.
- Build penuh dari PBF Indonesia.
- Satu file index dan manifest yang bisa diverifikasi.

### 7.2 Tidak masuk

- House number search.
- Pencarian berdasarkan kategori saja, misalnya "semua restoran", tanpa nama.
- Pencarian di luar Indonesia.
- Hasil berbentuk geometri.
- Edit data melalui API.
- Multi-region sharding.

## 8. Product requirements

| ID | Requirement | Prioritas | Ukuran keberhasilan |
|---|---|---|---|
| P-01 | Pengguna bisa mencari tempat berdasarkan nama, dengan bias lokasi opsional | Must | Query nama mengembalikan hasil berurutan dalam batas `limit` |
| P-02 | Pengguna bisa menemukan fitur bernama terdekat dari koordinat | Must | Titik di jalan bernama mengembalikan jalan atau tempat sekitar |
| P-03 | Pengguna bisa membuka tempat dari `place_id` atau `osm_ids` | Must | ID yang masih ada mengembalikan objek yang sama |
| P-04 | `place_id` stabil antar rebuild | Must | Rebuild PBF yang sama menghasilkan ID yang sama |
| P-05 | `display_name` dan `address` memakai hierarki administratif | Should | Tempat di dalam kecamatan punya parent yang benar, atau field kosong bila polygon tidak ada |
| P-06 | Index berupa satu file yang bisa dipromosikan dan di-rollback | Must | Serve hanya butuh satu path index, bukan koneksi DB |
| P-07 | Build deterministik dari PBF dan versi builder | Must | Index body yang sama menghasilkan SHA-256 yang sama |
| P-08 | Respons mengikuti subset Nominatim `jsonv2` | Should | Klien yang memakai field umum bisa membaca respons tanpa adapter besar |
| P-09 | Ada endpoint health, version, dan metrics | Must | Operasi bisa memeriksa status dan versi index |
| P-10 | Nama Indonesia dinormalisasi dan alias ikut dicari | Must | `Jl.`, `Jalan`, diakritik, dan alias menghasilkan kandidat yang sama |
| P-11 | Error dan limit API terdefinisi | Must | Input salah menghasilkan 400 dengan alasan yang bisa dibaca mesin |
| P-12 | Ada dokumentasi build, deploy, rebuild, dan rollback | Must | Operator bisa menjalankan prosedur tanpa membaca kode |

## 9. Metrik sukses

Semua angka di bawah adalah target awal untuk gate v1, bukan klaim yang sudah
terukur. Hasil pengukuran masuk ke [BENCHMARKS](BENCHMARKS.md).

| Metrik | Target awal | Cara ukur |
|---|---|---|
| Top-1 relevance | ≥ 80% | 300 query berlabel manual, stratified per jenis fitur |
| Top-5 relevance | ≥ 95% | Himpunan query yang sama |
| Search p95 | ≤ 50 ms | 20 RPS, warm cache, 4 vCPU, query campuran |
| Reverse p95 | ≤ 30 ms | 20 RPS, warm cache, 4 vCPU |
| Index Indonesia | ≤ 600 MB | Ukuran `places.idx` tanpa data opsional di luar budget |
| Idle RSS | ≤ 50 MB | Proses serve setelah index termap, tanpa traffic |
| Startup | ≤ 1 detik | Waktu sampai `/healthz` siap, index 600 MB, page cache dingin |
| Build Indonesia | ≤ 15 menit | 12 worker, 16 GB RAM, SSD, PBF sudah tersedia |
| Determinisme | 100% | Dua build dari PBF dan builder yang sama |
| Crash saat query valid | 0 | Benchmark dan fuzz input |

## 10. Constraint

- Data OSM berlisensi ODbL 1.0. Atribusi wajib terlihat di respons API dan
  dokumentasi.
- Kode berlisensi MIT.
- Tidak ada PBF, index, atau dump data di git.
- Target platform Linux amd64 dan arm64.
- Layanan v1 bersifat read-only dan single-node.
- Tidak ada API key di layanan. Pembatasan laju dilakukan di gateway.
- Tidak ada dependensi runtime database.
- Semua target performa harus diukur pada mesin referensi yang dicatat.

## 11. Fase rilis

| Fase | Isi | Gate |
|---|---|---|
| 0 | Dokumen fondasi dan kontrak | PRD, SRS, TRD, API, INDEX-FORMAT disetujui |
| 1 | Builder dan reader dengan fixture kecil | Format bisa ditulis, dibaca, dan divalidasi |
| 2 | Search, reverse, lookup | API berjalan di container dengan fixture |
| 3 | Build Indonesia dan benchmark | Target P-01 sampai P-12 lolos di mesin referensi |
| 4 | Kualitas ranking dan admin | Top-1 dan top-5 mencapai target |
| 5 | Rebuild terjadwal dan operasi | Rollback dan rebuild terbukti |

Urutan detail dan exit criteria ada di [ROADMAP](ROADMAP.md).

## 12. Pertanyaan terbuka

| Pertanyaan | Dampak | Status |
|---|---|---|
| Apakah polygon admin level 4 sampai 7 cukup, atau perlu level 8? | Ukuran index dan kualitas `address` | Perlu diuji pada sampel |
| Apakah reverse harus memprioritaskan jalan di atas POI pada jarak sama? | Relevansi reverse | Perlu data benchmark |
| Apakah `place_id` perlu string untuk klien yang tidak memakai JavaScript? | Kompatibilitas | 53-bit aman untuk JavaScript, tetap angka |
| Seberapa sering rebuild? | Operasi | Usulan bulanan, belum final |
| Apakah perlu dukungan `bounded=1` dan `viewbox` di v1? | API | Should, bukan Must |
| Apakah nama merek seperti Indomaret perlu dedup lintas cabang? | Ranking | Perlu strategi cluster, belum v1 |

## 13. Risiko produk

| Risiko | Dampak | Mitigasi |
|---|---|---|
| Kualitas ranking lebih sulit dari indexing | Target relevansi tidak tercapai | Query set berlabel, evaluasi per jenis fitur, tuning terpisah |
| Polygon admin mahal | Index membesar, build lambat | Simpan polygon opsional, ukur sebelum memasukkan |
| Data OSM tidak lengkap atau salah | Hasil salah | Tampilkan `osm_type` dan `osm_id`, sediakan lookup untuk audit |
| Scope creep ke Nominatim | Proyek tidak selesai | Non-goals dan gate API dijaga |
| Format index berubah berkali-kali | Klien dan operator terkena | Versioning, manifest, dan validasi ketat |
| Reverse jalan memakai centroid | Titik di tepi jalan panjang bisa 404 atau mendapat fitur lain | Ukur dengan label jalan panjang, dokumentasikan batas, tambah geometri hanya jika target gagal |
| Klaim performa tanpa data | Kepercayaan rusak | Semua angka diberi label target sampai diukur |
