# TRD: placeid

| Field | Value |
|---|---|
| Status | Draft |
| Versi | 0.1 |
| Tanggal | 2026-11-13 |
| Dokumen terkait | [SRS](SRS.md), [ARCHITECTURE](ARCHITECTURE.md), [INDEX-FORMAT](INDEX-FORMAT.md), [API](API.md) |

## 1. Tujuan teknis

TRD ini menjelaskan cara membangun `placeid` dengan biaya operasional kecil:
satu binary server, satu file index `mmap`, dan build batch dari PBF. Fokus
rancangan ada pada tiga hal:

1. Index yang bisa dibaca tanpa alokasi besar per request.
2. Build yang deterministik dan bisa diulang.
3. Ranking yang cukup baik untuk nama tempat Indonesia, bukan parity
   Nominatim.

Requirement yang mengikat ada di [SRS](SRS.md). Dokumen ini menjelaskan
mekanisme, bukan menambah requirement baru.

## 2. Batas sistem

```text
+-------------+       +------------------+       +------------------+
| OSM PBF     |  -->  | placeid-build    |  -->  | places.idx       |
| Indonesia   |       | Rust, batch      |       | + manifest.json  |
+-------------+       +------------------+       +------------------+
                                                          |
                                                          | mmap read-only
                                                          v
+-------------+       +------------------+       +------------------+
| HTTP client |  -->  | placeid-serve    |  -->  | Index reader     |
|             |       | Rust, axum/tokio |       | search, reverse  |
+-------------+       +------------------+       +------------------+
```

Di luar batas: gateway routeid, load balancer, dan sistem monitoring. Gateway
menangani autentikasi, rate limit, dan TLS. `placeid` hanya melayani HTTP di
jaringan internal.

## 3. Stack

| Komponen | Pilihan | Alasan |
|---|---|---|
| Bahasa | Rust stable | Kontrol memori, binary tunggal, tanpa runtime GC |
| HTTP | `axum` + `tokio` | Ringan, typed extractor, ekosistem matang |
| mmap | `memmap2` | Map read-only, tidak menyalin seluruh index ke heap |
| FST | `fst` | Prefix dan Levenshtein automaton, ukuran kecil |
| PBF | `osmpbf` | Parser Rust, bisa dipakai paralel |
| Parallel build | `rayon` | Parallelisme data untuk seleksi dan sorting |
| Zero-copy record | `bytemuck` | Baca record fixed-size dari slice |
| Normalisasi | `unicode-normalization` | NFKC dan diakritik |
| Serialization | `serde`, `serde_json` | Manifest dan respons API |
| CLI | `clap` | Kontrak flag yang jelas |
| Log | `tracing`, `tracing-subscriber` | Log terstruktur |
| Checksum | `sha2` | SHA-256 manifest dan index |
| Metrik | `metrics` + exporter Prometheus | Counter dan histogram |

Dependency baru harus punya alasan yang jelas. Jangan menambah crate untuk
fungsi yang bisa ditulis dalam puluhan baris dan diuji.

## 4. Struktur crate

Satu crate, dua binary:

```text
Cargo.toml
src/
  lib.rs
  build/
    mod.rs
    select.rs
    geometry.rs
    writer.rs
  index/
    mod.rs
    header.rs
    places.rs
    strings.rs
    fst.rs
    postings.rs
    grid.rs
    admin.rs
    validate.rs
  search/
    normalize.rs
    forward.rs
    reverse.rs
    rank.rs
  api/
    mod.rs
    search.rs
    reverse.rs
    lookup.rs
    errors.rs
  bin/
    placeid-build.rs
    placeid-serve.rs
```

`lib.rs` tidak boleh membuka HTTP atau membaca environment. Binary hanya
merangkai komponen. Ini memudahkan test fixture tanpa menjalankan server.

## 5. Pipeline build

### 5.1 Pass 1: kumpulkan fitur dan referensi

Scan node, way, dan relation. Simpan:

- Fitur node bernama beserta koordinatnya.
- Fitur way dan relation bernama dengan tag yang lolos seleksi.
- Daftar node ID untuk way bernama, agar centroid bisa dihitung.
- Daftar way ID untuk relation yang perlu geometri, termasuk admin.
- Tag admin: `boundary=administrative`, `admin_level`, `name`.
- Tag alamat pada fitur terpilih, untuk melengkapi `address`. Objek alamat
  tanpa nama tidak dimasukkan sebagai place.

Jangan menyimpan semua node. Hanya node ID yang dirujuk fitur terpilih.

### 5.2 Pass 2: resolusi koordinat

Scan node. Simpan koordinat hanya untuk node ID yang ada di set referensi dan
belum tersedia dari pass 1. Setelah pass ini, centroid way bisa dihitung dari
rata-rata koordinat node. Untuk polygon, simpan ring node ID dan koordinatnya.

Node yang hilang karena berada di luar extract dilewati. Fitur yang tidak
punya koordinat valid tidak masuk index, dan jumlahnya dicatat di manifest.

### 5.3 Pass 3: geometri relation dan admin

Scan way untuk way ID yang dirujuk relation. Susun ring polygon dari member
way dengan role `outer` dan `inner`. Hitung bbox dan centroid. Polygon admin
yang tidak tertutup atau tidak valid dicatat dan dilewati, bukan dipaksa
menjadi polygon.

Untuk relation POI non-admin, centroid dihitung dari semua member yang punya
koordinat. Jika tidak ada, fitur dilewati.

### 5.4 Normalisasi dan dedup

- Normalisasi nama untuk token FST.
- Simpan nama asli dan varian bahasa.
- Dedup berdasarkan `(osm_type, osm_id)`. Tidak ada merge lintas tipe di v1.
- Urutkan fitur berdasarkan `place_id` sebelum menulis record.
- Postings diurutkan menaik dan didedup.
- Token diurutkan secara leksikografis sebelum FST dibangun.

### 5.5 Penulisan

Tulis ke `places.idx.tmp` dan `places.idx.json.tmp`, hitung checksum, lalu
`rename` atomik. Manifest ditulis setelah index selesai. Jika proses mati di
tengah, file `.tmp` boleh tertinggal, tetapi index lama tidak tersentuh.

## 6. Index

Layout lengkap ada di [INDEX-FORMAT](INDEX-FORMAT.md). Ringkasnya:

| Section | Isi |
|---|---|
| Header | Magic, versi, jumlah, offset section, bbox, checksum |
| Places | Record fixed-size 64 byte, diurutkan `place_id` |
| Class/type dictionary | String kategori untuk API |
| Strings | Nama asli, nama admin, label |
| Name variants | Daftar `(lang, string)` per place/admin |
| Token FST | Token ternormalisasi ke offset dan panjang postings |
| Postings | Row index place, `u32`, terurut |
| Name FST | Nama lengkap ternormalisasi ke row index |
| Grid | Sel spasial ke row index |
| Admin | Record admin dan parent |
| Admin polygons | Opsional, untuk point-in-polygon saat build |
| Trailer | SHA-256 index body |

Record place menyimpan `place_id`, offset nama, `kind`, koordinat, importance,
`admin_id`, bbox, class/type ID, dan offset varian nama. `display_name` tidak
disimpan penuh. String disusun saat query dari nama fitur dan rantai admin.

## 7. Alur query

### 7.1 Forward search

```text
q
  -> validasi dan normalisasi
  -> exact lookup di name FST
  -> token lookup di token FST
  -> irisan postings
  -> ambil record kandidat
  -> skor
  -> top-K heap
  -> susun respons
```

Aturan kandidat:

- Exact match dicek lebih dulu. Jika ada hasil yang cukup, token scan bisa
  dilewati.
- Untuk multi-token, ambil postings token paling jarang, lalu intersect dengan
  token lain. Ini menghindari scan token umum seperti `jalan`.
- Batas kandidat 100.000 row per query. Jika terlampaui, ranker hanya memakai
  token paling jarang dan memberi penalti cakupan.
- `limit` maksimum 20. Top-K memakai heap ukuran `limit`, bukan sort seluruh
  kandidat.

### 7.2 Skor

Skor awal:

```text
score =
    3.0 * exact
  + 1.5 * prefix
  + 1.0 * token_coverage
  + 0.5 * importance
  + 0.5 * proximity
```

`exact` dan `prefix` bernilai 0 atau 1. `token_coverage` adalah rasio token
query yang cocok. `importance` dinormalisasi ke 0 sampai 1 saat build.
`proximity` hanya dipakai jika `lat`/`lon` ada:

```text
proximity = exp(-distance_m / 25000)
```

`importance` dihitung saat build dari kelas fitur dan sinyal OSM. Urutan
pengaruh awal: admin dan `place` di atas `highway`, `highway` di atas POI,
POI di atas bangunan. Bonus kecil diberikan untuk `population`, `wikidata`,
`capital`, dan `brand`. Nilai di-clamp 0 sampai 1. Ambang dan bonus adalah
titik tuning, bukan kontrak format.

Bobot disimpan di konfigurasi server, bukan di index. Tie-break memakai
`importance` menurun, lalu `place_id` menaik.

### 7.3 Reverse

```text
lat, lon
  -> cell grid
  -> cell sekitar dalam radius
  -> kandidat row
  -> jarak haversine
  -> pilih terdekat
  -> susun address dari admin
```

Grid memakai sel sekitar 0,01 derajat. Untuk radius 1 km, server memeriksa sel
pusat dan delapan tetangga. Untuk radius lebih besar, jumlah sel dihitung dari
radius. Kandidat diurutkan berdasarkan jarak, lalu prioritas jenis:

1. `place` dan `boundary`
2. `highway`
3. POI dan bangunan

Prioritas hanya dipakai jika jarak selisih kurang dari 25 meter. Di luar itu,
jarak murni yang menang.

### 7.4 Admin hierarchy

Saat build, setiap place diberi `admin_id` terdalam yang mengandung centroid.
`admin_id` dialokasikan setelah polygon admin diurutkan berdasarkan
`(admin_level, osm_id)`. Pencarian parent memakai point-in-polygon pada
polygon admin level 4 sampai 7.
Jika beberapa polygon pada level yang sama mengandung titik, builder memilih
polygon dengan luas terkecil; jika luas sama, `admin_id` terkecil. Jika tidak
ada polygon yang cocok, `admin_id` bernilai 0 dan `address` kosong.

Saat query, rantai admin disusun dari `admin_id` ke parent. Nama admin dipilih
berdasarkan `accept-language`, lalu `name` sebagai fallback. Tidak ada nama
parent yang dikarang. Untuk index Indonesia, `display_name` menambahkan
`Indonesia` di akhir bila belum ada.

## 8. Normalisasi

Aturan yang sama dipakai builder dan server:

1. Unicode NFKC.
2. Lowercase.
3. Hapus diakritik.
4. Ganti tanda baca dengan spasi, kecuali yang punya arti.
5. Rapikan spasi.
6. Petakan singkatan:

| Input | Normalisasi |
|---|---|
| `Jl.`, `Jl` | `jalan` |
| `Kec.`, `Kec` | `kecamatan` |
| `Kab.`, `Kab` | `kabupaten` |
| `Kel.`, `Kel` | `kelurahan` |
| `Kp.`, `Kp` | `kampung` |
| `No.` | `nomor` |

Tokenisasi memakai pemisah spasi dan tanda baca. Token satu karakter yang
bukan angka dibuang. Nama asli tetap disimpan untuk output.

## 9. Konkurensi dan memori

- Index immutable setelah dimap. Tidak ada lock global saat query.
- `tokio` multi-thread melayani request. Pekerjaan CPU per query dibatasi oleh
  kandidat dan `limit`.
- Setiap request memakai buffer sendiri untuk hasil. Tidak ada cache query
  global di v1.
- `mmap` membuka file read-only. Halaman yang tidak disentuh tidak masuk RSS.
- Build boleh memakai memori lebih besar. Target build 16 GB; jika melebihi,
  build gagal dengan pesan yang jelas.
- `--max-concurrency` membatasi request berat agar latency ekor terkendali.

## 10. Kegagalan dan penanganan

| Kegagalan | Perilaku |
|---|---|
| Index hilang | Server exit non-zero saat start |
| Magic/versi salah | Server menolak index, exit non-zero |
| Section offset di luar batas | Validasi gagal, exit non-zero |
| Checksum mismatch saat verifikasi | Server menolak index |
| PBF korup | Builder exit non-zero, index lama utuh |
| Disk penuh saat build | File `.tmp` dihapus, index lama utuh |
| Polygon admin tidak valid | Polygon dilewati, dicatat di manifest |
| Node referensi hilang | Fitur dilewati, dihitung di manifest |
| Query terlalu umum | Kandidat dibatasi, hasil diberi penalti cakupan |
| Panic di handler | Koneksi ditutup, proses tetap hidup bila memakai catch layer |

Server tidak boleh panic karena input HTTP. Semua parsing angka, rentang
koordinat, dan panjang string harus divalidasi.

## 11. Build dan rilis

- `cargo build --release` menghasilkan `placeid-build` dan `placeid-serve`.
- Target binary memakai `x86_64-unknown-linux-musl` dan
  `aarch64-unknown-linux-musl` bila memungkinkan.
- Image container memakai base minimal, user non-root, root filesystem
  read-only, dan volume index read-only.
- Manifest disimpan bersama index dan dipromosikan sebagai satu unit.
- Versi builder masuk ke manifest. Index dari builder berbeda boleh dibaca
  selama versi format sama.
- Perubahan format wajib menaikkan `format_version` dan menambah test
  kompatibilitas baca.

## 12. Observability

| Sinyal | Isi |
|---|---|
| Log | Request ID, method, path, status, latency, result count |
| Metrik | Request total, error total, latency histogram, candidate count, index size |
| Health | Status index dan versi format |
| Version | Versi server dan checksum manifest |
| Build | Durasi per pass, jumlah fitur, jumlah fitur dilewati, ukuran output |

Jangan memakai teks query sebagai label metrik. Kardinalitasnya tidak terbatas.

## 13. Keamanan

- Index dianggap input tepercaya yang dibangun operator, tetapi parser tetap
  memvalidasi batas.
- `unsafe` hanya boleh dipakai untuk mmap dan pembacaan record fixed-size.
  Setiap blok `unsafe` harus punya komentar alasan dan test.
- Parameter HTTP dibatasi: panjang query 256 byte, `limit` 20, `radius` 10 km,
  `osm_ids` 50.
- Tidak ada akses filesystem dari request.
- Tidak ada panggilan jaringan keluar dari server.
- Error tidak memuat path absolut atau stack trace.
- Dependency diaudit dengan `cargo audit` di CI.

## 14. Alternatif yang dipertimbangkan

| Alternatif | Kelebihan | Alasan tidak dipilih untuk v1 |
|---|---|---|
| SQLite FTS5 | Cepat dibangun, SQL familiar, file tunggal | Menambah C dependency dan query planner yang tidak sepenuhnya dikontrol. Tetap menjadi fallback bila ranking custom melebihi budget |
| Tantivy | Full-text matang, scoring fleksibel | Ditujukan untuk search engine umum, multi-file, dan tidak punya grid spasial/admin |
| PostgreSQL + PostGIS + Nominatim | Fitur lengkap, komunitas besar | Service database, RAM besar, impor panjang, dan scope terlalu luas |
| Pelias | Pipeline lengkap | Banyak komponen dan service, sulit dioperasikan untuk satu negara |
| Index in-memory | Query cepat | RSS membesar dan startup lambat; bertentangan dengan target footprint |

Keputusan memakai index custom dicatat di
[ADR 0002](decisions/0002-custom-mmap-index.md). Jika benchmark menunjukkan
bahwa SQLite FTS5 memenuhi target dengan biaya lebih rendah, keputusan harus
ditinjau ulang. Membangun format custom bukan tujuan produk.

## 15. Risiko teknis

| Risiko | Dampak | Mitigasi |
|---|---|---|
| Ranking terlalu sederhana | Hasil tidak relevan | Query set berlabel, evaluasi per kategori, tuning bobot |
| Kandidat token umum meledak | Latency tidak stabil | Pilih token jarang, batas kandidat, exact match lebih dulu |
| Polygon admin mahal | Index dan build membesar | Section opsional, ukur sebelum mengaktifkan |
| Format custom sulit dirawat | Bug parser dan kompatibilitas | Spesifikasi, fuzz, validasi, versi format |
| Build tidak deterministik | Rollback dan audit sulit | Sorting eksplisit, larangan iterasi hash map |
| Dependency PBF berubah | Build rusak | Pin versi, fixture PBF, test integrasi |
| Klaim performa tanpa data | Keputusan salah | Benchmark sebelum rilis, label target |

## 16. Traceability teknis

| Requirement | Komponen |
|---|---|
| FR-001, FR-013, FR-014 | `build/writer.rs`, `index/validate.rs` |
| FR-002, FR-005 | `build/select.rs`, `search/normalize.rs` |
| FR-003 | `index/places.rs`, ADR 0003 |
| FR-004, FR-006 | `search/forward.rs`, `search/rank.rs` |
| FR-007 | `search/reverse.rs`, `index/grid.rs` |
| FR-008 | `api/lookup.rs` |
| FR-009 | `build/geometry.rs`, `build/admin.rs` |
| FR-010, FR-012 | `api/errors.rs`, `api/mod.rs` |
| FR-011 | `api/mod.rs`, metrik |
| FR-015, FR-016 | `index/mod.rs`, `tracing` |
| NFR-001 sampai NFR-006 | Benchmark, lihat [BENCHMARKS](BENCHMARKS.md) |
| NFR-013 | Query set berlabel, `search/rank.rs` |
