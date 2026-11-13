# SRS: placeid

| Field | Value |
|---|---|
| Status | Draft |
| Versi | 0.1 |
| Tanggal | 2026-11-13 |
| Requirement language | RFC 2119 |
| Dokumen terkait | [PRD](PRD.md), [TRD](TRD.md), [API](API.md), [INDEX-FORMAT](INDEX-FORMAT.md) |

## 1. Tujuan dan ruang lingkup

Dokumen ini menetapkan perilaku `placeid` yang bisa diuji. Fokusnya adalah
batas sistem, requirement fungsional, requirement nonfungsional, antarmuka
eksternal, dan cara memverifikasi bahwa requirement terpenuhi.

Requirement di sini berlaku untuk v1: index Indonesia, forward search,
reverse nearest, lookup, dan endpoint operasional. Fitur di luar itu bukan
bagian dari kontrak v1.

Kata `MUST`, `MUST NOT`, `SHOULD`, `SHOULD NOT`, dan `MAY` mengikuti RFC 2119.
`MUST` berarti syarat lulus v1. `SHOULD` berarti boleh ditunda hanya dengan
alasan yang dicatat. `MAY` berarti opsional.

## 2. Definisi

| Istilah | Arti |
|---|---|
| Fitur | Objek OSM bertipe node, way, atau relation yang masuk seleksi |
| Place | Fitur yang punya nama dan bisa dikembalikan API |
| Forward search | Pencarian berdasarkan teks nama |
| Reverse | Pencarian fitur berdasarkan koordinat |
| Lookup | Pengambilan fitur berdasarkan identifier |
| Index | File `places.idx` yang dipetakan read-only oleh server |
| Manifest | File JSON berisi provenance, versi, jumlah, dan checksum |
| Builder | Binary `placeid-build` yang mengubah PBF menjadi index |
| Server | Binary `placeid-serve` yang melayani HTTP |
| Admin hierarchy | Rantai wilayah administratif dari fitur ke parent |
| `place_id` | Identifier 53-bit turunan `(osm_type, osm_id)` |

## 3. Konteks sistem

```text
OSM PBF  ->  placeid-build  ->  places.idx + manifest
                                      |
                                      v
Client  ->  HTTP  ->  placeid-serve  ->  mmap index
```

Server tidak membaca PBF, tidak menulis index, dan tidak memerlukan database.
Build dan serve adalah dua binary dari crate yang sama.

## 4. Requirement fungsional

### FR-001 Build index dari PBF

Builder **MUST** menerima path PBF, path index output, dan path manifest.
Builder **MUST** menulis ke file sementara lalu mengganti nama secara atomik
setelah seluruh index dan manifest selesai. Jika build gagal, file index lama
**MUST NOT** rusak atau terhapus.

Acceptance:

- Perintah dengan PBF valid menghasilkan index dan manifest, exit code 0.
- PBF tidak ada atau rusak menghasilkan exit code non-zero dan pesan error.
- Setelah build gagal, path index lama masih bisa dibuka server.

### FR-002 Seleksi fitur

Builder **MUST** menyertakan fitur yang punya nama dan termasuk kategori yang
didukung: `place`, `boundary=administrative`, `highway` bernama, `amenity`,
`shop`, `tourism`, `leisure`, `office`, `healthcare`, `education`, `transport`,
`natural`, `waterway`, `railway`, `aeroway`, `man_made`, `historic`, dan
`building` bernama yang punya kategori. Builder **MUST NOT** menyertakan objek
alamat tanpa nama sebagai place mandiri. Tag `addr:*` pada fitur terpilih
**MAY** dipakai untuk melengkapi `address`, tetapi bukan untuk pencarian nomor
rumah.

Builder **MUST** mencatat jumlah fitur per kategori di manifest.

Acceptance:

- Fixture dengan fitur bernama dan tidak bernama hanya mengembalikan yang
  bernama.
- Fitur `highway=residential` tanpa nama tidak muncul di hasil.
- Objek `addr:housenumber` tanpa nama tidak muncul sebagai hasil search.
- Manifest memuat jumlah per kategori dan total.

### FR-003 Identitas place

Builder **MUST** menurunkan `place_id` dengan rumus:

```text
place_id = (type_code << 51) | osm_id
type_code: node=1, way=2, relation=3
```

`osm_id` **MUST** kurang dari `2^51`. Jika ada `osm_id` di luar batas,
builder **MUST** gagal dengan error yang menyebut objek tersebut. `place_id`
**MUST** unik di dalam index. `place_id` **MUST** sama untuk objek OSM yang
sama pada rebuild berikutnya.

Acceptance:

- Dua build dari PBF yang sama menghasilkan `place_id` yang sama.
- `place_id` maksimum ≤ `2^53 - 1`, sehingga aman dibaca sebagai angka
  JavaScript.
- Dua fitur tidak bisa punya `place_id` yang sama.

### FR-004 Forward search

Server **MUST** menyediakan pencarian berdasarkan parameter `q`.
`q` **MUST** required dan **MUST** ditolak dengan 400 jika kosong atau hanya
berisi spasi. `limit` **MUST** dibatasi 1 sampai 20 dengan default 5.
Parameter `lat` dan `lon` **MAY** dipakai sebagai bias lokasi. `viewbox` dan
`bounded` **MAY** dipakai untuk membatasi area.

Hasil **MUST** diurutkan berdasarkan skor menurun. Jika skor sama, urutan
**MUST** memakai `importance` menurun lalu `place_id` menaik agar deterministik.
Hasil **MUST NOT** mengandung `place_id` duplikat.

Acceptance:

- Query nama yang ada mengembalikan array JSON dengan field sesuai API.
- `limit=0`, `limit=21`, atau `q=` kosong menghasilkan 400.
- Query yang sama dengan input sama menghasilkan urutan yang sama.

### FR-005 Normalisasi dan alias

Server **MUST** menormalisasi query dan nama index dengan aturan yang sama:
Unicode NFKC, huruf kecil, penghapusan diakritik, penyatuan spasi, dan
pemetaan singkatan Indonesia. Pemetaan **MUST** mencakup `Jl.` dan `Jalan`
menjadi `jalan`, `Kec.` menjadi `kecamatan`, `Kab.` menjadi `kabupaten`,
`Kel.` menjadi `kelurahan`, dan `Kp.` menjadi `kampung`.

Builder **MUST** mengindeks `name`, `name:id`, `name:en`, `alt_name`,
`old_name`, `official_name`, `short_name`, dan `brand` bila ada. Nama asli
**MUST** dipertahankan untuk output.

Acceptance:

- `Jl. Malioboro` dan `Jalan Malioboro` menghasilkan kandidat yang sama.
- Nama dengan diakritik bisa ditemukan tanpa diakritik.
- Output menampilkan nama asli, bukan bentuk normalisasi.

### FR-006 Ranking

Ranker **MUST** memakai sinyal exact match, prefix match, cakupan token,
`importance`, dan jarak bila `lat`/`lon` diberikan. Untuk query multi-token,
ranker **MUST** mencoba kecocokan semua token terlebih dahulu. Jika tidak ada
hasil, ranker **MAY** mencoba kecocokan sebagian dengan penalti.

Ranker **MUST** deterministik untuk input yang sama. Bobot **MUST** bisa
diubah tanpa mengubah format index.

Acceptance:

- Exact match mengalahkan prefix match pada kondisi lain yang sama.
- Nama dengan semua token cocok mengalahkan nama yang hanya cocok satu token.
- Perubahan bobot tidak memerlukan rebuild index.

### FR-007 Reverse geocoding

Server **MUST** menerima `lat` dan `lon` yang valid. Default `radius` adalah
1000 meter; nilai yang diterima 1 sampai 10000 meter. Server **MUST**
mengembalikan fitur bernama terdekat di dalam radius. Jika tidak ada hasil,
server **MUST** mengembalikan 404 dengan alasan `not_found`.

Reverse **MUST NOT** melakukan interpolasi nomor rumah. Reverse **MAY**
mengembalikan `address` bila admin hierarchy tersedia.

Acceptance:

- Titik di jalan bernama mengembalikan jalan atau fitur sekitar.
- Titik di laut tanpa fitur dalam radius mengembalikan 404.
- `radius=0` atau `radius=10001` menghasilkan 400.

### FR-008 Lookup

Server **MUST** menerima `osm_ids` berupa daftar dipisah koma dengan awalan
`N`, `W`, atau `R`, contoh `N123,W456,R789`. Server **MAY** menerima
`place_ids` berupa daftar angka. Jumlah ID per request **MUST** dibatasi 50.

Server **MUST** mengembalikan hanya ID yang ditemukan, dalam urutan yang sama
dengan permintaan. ID yang tidak ditemukan **MUST** dihilangkan, bukan
diganti dengan error, selama request valid.

Acceptance:

- Lookup dua ID yang ada mengembalikan dua objek berurutan.
- ID tidak dikenal dihilangkan tanpa mengubah urutan ID lain.
- Lebih dari 50 ID menghasilkan 400.

### FR-009 Hierarki administratif dan display_name

Jika polygon administratif tersedia, server **MUST** menyusun `address` dari
parent yang mengandung titik fitur. Jika fitur punya tag `addr:*`, field yang
relevan **MAY** diisi dari tag tersebut. `display_name` **MUST** berupa nama
fitur diikuti parent dari level terdekat ke terjauh, dipisah koma. Untuk index
Indonesia, server **MUST** menambahkan `Indonesia` di akhir bila belum ada.
Jika parent tidak tersedia, server **MUST NOT** mengarang nama parent.

Server **MUST** mendukung `accept-language` untuk memilih nama `name:id` atau
`name:en` bila tersedia. Jika tidak tersedia, server **MUST** memakai `name`.

Acceptance:

- Fitur di dalam kecamatan menampilkan kecamatan dan kabupaten yang benar.
- Fitur di luar polygon yang diketahui tidak menampilkan parent palsu.
- `accept-language=id` memilih nama Indonesia bila ada.

### FR-010 Format respons

Server **MUST** mengembalikan JSON UTF-8 dengan `Content-Type:
application/json; charset=utf-8`. Field minimum untuk search, reverse, dan
lookup adalah `place_id`, `licence`, `osm_type`, `osm_id`, `lat`, `lon`,
`class`, `type`, `name`, `display_name`, `importance`, dan `boundingbox`.
Field `address` **MUST** ada bila `addressdetails=1`.

Koordinat **MUST** dikirim sebagai string desimal untuk kompatibilitas
Nominatim. `place_id` dan `osm_id` **MUST** dikirim sebagai angka.

Acceptance:

- Respons dapat diparse sebagai JSON tanpa field yang hilang.
- `lat` dan `lon` berbentuk string, bukan number.
- `licence` menyebut ODbL.

### FR-011 Endpoint operasional

Server **MUST** menyediakan:

- `/healthz`: 200 jika index sudah termap dan header valid.
- `/version`: versi server, versi format index, dan ringkasan manifest.
- `/metrics`: metrik Prometheus.

`/healthz` **MUST NOT** melakukan pencarian atau menyentuh seluruh halaman
index. `/version` **MUST** menyebut checksum manifest yang sedang dipakai.

Acceptance:

- Server tanpa index tidak lulus `/healthz`.
- `/version` berubah setelah index baru dipromosikan.
- `/metrics` bisa diparse Prometheus.

### FR-012 Error dan limit

Server **MUST** mengembalikan error JSON dengan field `error` dan `reason`
untuk input tidak valid. Kode status yang dipakai:

| Kondisi | Status |
|---|---|
| Parameter hilang atau tidak valid | 400 |
| Format tidak didukung | 400 |
| Hasil tidak ditemukan | 404 |
| Method tidak didukung | 405 |
| Body terlalu besar | 413 |
| Terlalu banyak request | 429, dipasang gateway |
| Index tidak siap | 503 |

Server **MUST NOT** membocorkan path filesystem atau stack trace di respons.

Acceptance:

- Semua error yang diuji punya `error` dan `reason`.
- Tidak ada stack trace di body.
- 405 menyertakan header `Allow`.

### FR-013 Manifest dan validasi

Builder **MUST** menulis manifest JSON berisi: versi format, versi builder,
SHA-256 PBF, SHA-256 index body, ukuran file, jumlah fitur, jumlah nama,
jumlah admin, bounding box, dan waktu build. Server **MUST** menolak index
dengan magic atau versi format yang tidak dikenal.

Server **MAY** memverifikasi checksum penuh saat startup jika flag
`--verify-checksum` diberikan. Tanpa flag, server **MUST** memvalidasi header,
batas section, dan trailer.

Acceptance:

- Index dengan magic salah ditolak dengan exit non-zero.
- Manifest dengan checksum berbeda ditolak saat verifikasi.
- Server bisa start tanpa membaca seluruh file.

### FR-014 Build deterministik

Builder **MUST** mengurutkan fitur dan postings dengan aturan yang stabil.
Builder **MUST NOT** bergantung pada urutan iterasi hash map. Untuk PBF dan
versi builder yang sama, index body **MUST** identik. Manifest **MAY**
berbeda pada `built_at`.

Acceptance:

- Dua build berturut-turut menghasilkan SHA-256 index body yang sama.
- Perbedaan hanya pada `built_at` di manifest.

### FR-015 Serve read-only

Server **MUST** membuka index dengan mode read-only. Server **MUST NOT**
menulis ke file index, membuat file samping, atau memodifikasi manifest.
Server **MUST NOT** memproses PBF saat runtime.

Acceptance:

- Index dengan permission read-only bisa dipakai.
- Tidak ada file baru di direktori index setelah server berjalan.
- Server tidak membuka path PBF.

### FR-016 Logging

Server **MUST** menulis log terstruktur yang memuat request ID, method, path,
status, latency, dan jumlah hasil. Secara default, server **MUST NOT** menulis
`q`, `lat`, `lon`, atau `place_ids` ke log. Log query hanya boleh aktif dengan
flag debug eksplisit.

Acceptance:

- Log normal tidak memuat teks query.
- Setiap request punya request ID.
- Log debug memuat query dan diberi penanda.

## 5. Requirement nonfungsional

| ID | Requirement | Acceptance |
|---|---|---|
| NFR-001 | Search p95 **MUST** ≤ 50 ms pada 20 RPS, warm cache, 4 vCPU | Benchmark 10 menit, query campuran, p95 dihitung dari histogram |
| NFR-002 | Reverse p95 **MUST** ≤ 30 ms pada 20 RPS, warm cache, 4 vCPU | Benchmark yang sama, subset reverse |
| NFR-003 | Index Indonesia **MUST** ≤ 600 MB tanpa data opsional di luar budget | Ukuran file dicatat di manifest dan benchmark |
| NFR-004 | Idle RSS **MUST** ≤ 50 MB setelah index termap | `ps` atau `smem` setelah 60 detik tanpa traffic |
| NFR-005 | Startup **MUST** ≤ 1 detik sampai `/healthz` siap | Index 600 MB, page cache dingin, tanpa `--verify-checksum`, diukur 5 kali |
| NFR-006 | Build Indonesia **MUST** ≤ 15 menit pada 12 worker, 16 GB RAM, SSD | Waktu wall-clock dari PBF ke manifest |
| NFR-007 | Binary **MUST** berjalan di Linux amd64 dan arm64 | Build dan smoke test di kedua arsitektur |
| NFR-008 | Parser index **MUST** menolak offset dan panjang di luar batas | Fuzz dan test fixture rusak |
| NFR-009 | Server **MUST** memberi metrik request, latency, error, dan ukuran index | `/metrics` memuat counter dan histogram |
| NFR-010 | Query dan koordinat **MUST NOT** disimpan secara default | Audit log dan konfigurasi default |
| NFR-011 | Respons **MUST** membawa atribusi ODbL | Test kontrak memeriksa field `licence` |
| NFR-012 | Layanan v1 **MUST** berjalan sebagai proses tunggal tanpa database | Deploy container hanya butuh binary dan index |
| NFR-013 | Top-1 **MUST** ≥ 80% dan top-5 **MUST** ≥ 95% pada 300 query berlabel | Evaluasi memakai protokol [BENCHMARKS](BENCHMARKS.md), dilaporkan per kategori |

## 6. Antarmuka eksternal

| Antarmuka | Kontrak |
|---|---|
| HTTP API | [API.md](API.md) |
| File index | [INDEX-FORMAT.md](INDEX-FORMAT.md) |
| Manifest | JSON, field di FR-013 |
| Input PBF | `*.osm.pbf` dari Geofabrik atau ekstraksi Indonesia |
| CLI builder | `placeid-build --input <path> --output <path> --manifest <path>` |
| CLI server | `placeid-serve --index <path> --bind <host:port>` |
| Environment | `PLACEID_INDEX`, `PLACEID_ADDR`, `PLACEID_LOG`, `PLACEID_MAX_CONCURRENCY` |
| Metrik | Format teks Prometheus |

Nama CLI dan environment adalah target kontrak v1. Implementasi belum ada.

## 7. Data dan privasi

- Data berasal dari OpenStreetMap dan berlisensi ODbL 1.0.
- Index tidak boleh menyimpan data pribadi di luar yang sudah ada di OSM.
- Server tidak menyimpan query, koordinat, atau hasil pencarian.
- Log default tidak memuat teks query atau koordinat.
- Metrik hanya agregat, tanpa label query.
- Operator bertanggung jawab atas retensi log di tingkat gateway.

## 8. Traceability

| PRD | Requirement SRS |
|---|---|
| P-01 Forward search | FR-004, FR-005, FR-006, NFR-001, NFR-013 |
| P-02 Reverse | FR-007, NFR-002 |
| P-03 Lookup | FR-008 |
| P-04 Stable ID | FR-003, FR-014 |
| P-05 Admin hierarchy | FR-009 |
| P-06 Index artifact | FR-001, FR-013, FR-015, NFR-003 |
| P-07 Deterministic build | FR-014 |
| P-08 Nominatim subset | FR-010, FR-012 |
| P-09 Operational endpoints | FR-011 |
| P-10 Normalization | FR-005, FR-006, NFR-013 |
| P-11 Errors and limits | FR-012 |
| P-12 Operations docs | [OPERATIONS](OPERATIONS.md), NFR-012 |

## 9. Verifikasi

| Lapisan | Metode |
|---|---|
| Unit | Normalisasi, ranking, encoding `place_id`, parser FST dan postings |
| Fixture | PBF kecil dengan node, way, relation, admin, dan alamat |
| Integrasi | Build fixture, start server, panggil semua endpoint |
| Kontrak | Bandingkan field dan tipe respons dengan [API](API.md) |
| Fuzz | Parser index dan parameter HTTP |
| Benchmark | Protokol [BENCHMARKS](BENCHMARKS.md) |
| Relevansi | 300 query berlabel manual, stratified |
| Operasi | Rebuild, promote, rollback di container |

Definition of done v1: semua FR dan NFR lulus, benchmark tercatat, dan tidak
ada requirement `MUST` yang ditunda tanpa ADR.
