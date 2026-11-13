# placeid

Geocoder mandiri untuk Indonesia dari OpenStreetMap. Target bentuknya satu
crate Rust dengan dua binary, builder batch dan server HTTP, satu file index
`mmap`, dan tanpa service database.

> **English:** placeid is a self-hosted OpenStreetMap geocoder for Indonesia.
> It resolves place names, performs nearest-feature reverse geocoding, and
> exposes stable place identifiers. The design target is a small operational
> footprint: one Rust crate with two binaries, one server process, one
> memory-mapped index file, and no database service.

Status: **fondasi dokumen, belum ada kode**. Dokumen di repo ini menetapkan
batas produk, requirement, kontrak API, format index, backlog, riset metode,
dan rencana verifikasi. Angka performa di dokumen adalah target awal, bukan
hasil pengukuran, kecuali disebut lain.

Repo: <https://github.com/arifsans/placeid> (private). Clone, issue, commit,
dan push memakai GitHub CLI. Alur lengkap ada di
[CONTRIBUTING.md](CONTRIBUTING.md#repo-dan-github-cli).

## Masalah

Aplikasi yang butuh pencarian tempat di Indonesia biasanya memakai layanan
hosted dengan biaya per request, atau menjalankan Nominatim lengkap yang
memerlukan PostgreSQL, beberapa GB RAM, dan proses impor panjang. Untuk
kebutuhan yang sempit, yaitu mencari nama tempat, POI, jalan, dan mencari
fitur terdekat, sebagian besar kapasitas itu tidak terpakai.

`placeid` mengambil batas masalah yang lebih kecil:

- Data Indonesia saja pada v1.
- Nama tempat, POI, jalan, dan wilayah administratif.
- Forward search, reverse nearest, dan lookup ID.
- Rebuild penuh dari file PBF, bukan update diff.
- Satu file index yang bisa dipromosikan dan di-rollback sebagai artefak.

## Korpus yang diukur

Sensus berikut dijalankan pada 2026-11-13 terhadap extract Indonesia yang
dipakai routeid, `data/indonesia-latest.osm.pbf`. Angka ini adalah isi korpus,
bukan hasil index `placeid`.

| Objek | Total | Bernama | Punya `addr:housenumber` |
|---|---:|---:|---:|
| Node | 277.198.053 | 346.846 | 39.386 |
| Way | 50.921.496 | 878.076 | 67.702 |
| Relation | 71.229 | 39.027 | 0 |
| Total | 328.190.778 | 1.263.949 | 107.088 |

Jumlah nama unik mentah 645.238 dengan total 21.243.045 byte teks nama,
sekitar 21,2 MB. Setelah normalisasi menjadi 612.877 nama unik. Artinya,
bagian teks bukan bagian terbesar dari index. Ongkos utama ada di record,
postings, grid spasial, dan opsional polygon administratif.

Konsekuensi produk: geocoding nomor rumah tidak masuk v1. Hanya 107.088 objek
yang punya `addr:housenumber`, tersebar tidak merata, dan tidak cukup untuk
mengklaim akurasi alamat. Rincian token, duplikasi nama, ukuran FST, dan
occupancy grid ada di
[docs/research/LIGHTWEIGHT-METHODS.md](docs/research/LIGHTWEIGHT-METHODS.md).

## Scope v1

- `GET /search?q=...` untuk nama tempat, POI, jalan, dan wilayah.
- `GET /reverse?lat=...&lon=...` untuk fitur bernama terdekat.
- `GET /lookup?osm_ids=...` dan lookup `place_id`.
- Hierarki administratif untuk `display_name` dan `address`, bila polygon
  tersedia.
- Normalisasi nama Indonesia: huruf kecil, diakritik, singkatan umum seperti
  `Jl.`, `Kec.`, `Kab.`, `Kel.`, `Kp.`.
- Alias dari `name:id`, `name:en`, `alt_name`, `old_name`, `official_name`,
  `short_name`, dan `brand`.
- Endpoint operasional: `/healthz`, `/version`, `/metrics`.

## Non-goals v1

- Parity penuh dengan Nominatim.
- Geocoding nomor rumah dan interpolasi alamat.
- Update diff harian atau streaming replication.
- Routing, map matching, isochrone, atau polygon output.
- Cakupan dunia. Index v1 dibangun untuk Indonesia.
- Menjadi layanan publik multi-tenant dengan billing dan SLA.

## Bentuk teknis yang dituju

```text
PBF Indonesia
    |
    v
placeid-build  (Rust, batch)
    |
    v
places.idx + places.idx.json
    |
    v
placeid-serve  (Rust, HTTP, mmap read-only)
```

Index menyimpan record fitur, string asli, token FST, postings, name FST,
grid spasial, dan opsional polygon administratif. `place_id` diturunkan dari
`(osm_type, osm_id)` dan aman disimpan sebagai angka JavaScript 53-bit. Detail
kontrak ada di [docs/INDEX-FORMAT.md](docs/INDEX-FORMAT.md) dan
[docs/decisions/0003-place-id.md](docs/decisions/0003-place-id.md).

## Dokumentasi

| Dokumen | Isi |
|---|---|
| [docs/PRD.md](docs/PRD.md) | Masalah, pengguna, scope, requirement produk, metrik sukses |
| [docs/SRS.md](docs/SRS.md) | Requirement fungsional dan nonfungsional yang bisa diuji |
| [docs/TRD.md](docs/TRD.md) | Rancangan teknis, pipeline build, ranking, risiko |
| [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) | Diagram konteks, container, alur build dan query |
| [docs/API.md](docs/API.md) | Kontrak HTTP dan kompatibilitas subset Nominatim |
| [docs/INDEX-FORMAT.md](docs/INDEX-FORMAT.md) | Layout file index, validasi, versioning |
| [docs/BENCHMARKS.md](docs/BENCHMARKS.md) | Protokol benchmark dan format pelaporan hasil |
| [docs/OPERATIONS.md](docs/OPERATIONS.md) | Build, deploy, rebuild, rollback, troubleshooting |
| [docs/ROADMAP.md](docs/ROADMAP.md) | Fase pekerjaan dan exit criteria |
| [docs/BACKLOG.md](docs/BACKLOG.md) | Task yang bisa dilacak, ukuran, dependensi, acceptance |
| [docs/research/LIGHTWEIGHT-METHODS.md](docs/research/LIGHTWEIGHT-METHODS.md) | Riset metode ringan dan akurat, bukti, rekomendasi |
| [docs/GLOSSARY.md](docs/GLOSSARY.md) | Istilah teknis yang dipakai di repo |
| [docs/decisions/](docs/decisions/) | Architecture Decision Records |

Indeks dokumentasi dan aturan perubahan ada di
[docs/README.md](docs/README.md).

## Lisensi

Kode: MIT, lihat [LICENSE](LICENSE). Data OpenStreetMap: ODbL 1.0. Respons API
dan dokumentasi wajib membawa atribusi ke OpenStreetMap contributors.
