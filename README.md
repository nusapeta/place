# Nusapeta Place

Geocoder mandiri untuk Indonesia dari OpenStreetMap. Target bentuknya satu
crate Rust dengan dua binary, builder batch dan server HTTP, satu file index
`mmap`, dan tanpa service database.

> **English:** Nusapeta Place is a self-hosted OpenStreetMap geocoder for Indonesia.
> It resolves place names, performs nearest-feature reverse geocoding, and
> exposes stable place identifiers. The design target is a small operational
> footprint: one Rust crate with two binaries, one server process, one
> memory-mapped index file, and no database service.

Status: **v1 kandidat rilis**. Builder, server, format index, dataset uji,
container, metrik, alert, dan prosedur promote/rollback sudah ada. Angka
performa di [docs/BENCHMARKS.md](docs/BENCHMARKS.md) adalah hasil pengukuran
pada mesin uji, bukan target. Catatan rilis:
[docs/releases/0.1.0.md](docs/releases/0.1.0.md).

## Mulai cepat

```bash
export PATH="$HOME/.cargo/bin:$PATH"
cargo build --release --bins

target/release/nusa-place-build \
  --input data/indonesia-latest.osm.pbf \
  --output /data/places.idx \
  --manifest /data/places.idx.json \
  --verify

target/release/nusa-place \
  --index /data/places.idx \
  --manifest /data/places.idx.json \
  --bind 127.0.0.1:8090
```

Uji endpoint:

```bash
curl 'http://127.0.0.1:8090/search?q=monas&limit=3'
curl 'http://127.0.0.1:8090/reverse?lat=-6.175392&lon=106.827153'
curl 'http://127.0.0.1:8090/lookup?osm_ids=N1308678148'
curl 'http://127.0.0.1:8090/healthz'
curl 'http://127.0.0.1:8090/metrics'
```

Build penuh Indonesia memerlukan PBF dan RAM puncak sekitar 8 GiB. Index
final 141.997.032 B untuk 1.226.755 fitur. Container memakai port 8090 dan
mengharapkan index di `/data`.

## Toolchain

Toolchain dipin di `rust-toolchain.toml`: Rust `1.99.1` dengan `rustfmt` dan
`clippy`, profil minimal. Pasang rustup, lalu jalankan:

```bash
rustup show
cargo fmt --version
cargo clippy --version
```

`rustup show` harus menampilkan `1.99.1-x86_64-unknown-linux-gnu` (atau
toolchain host yang setara). Tanpa rustup, file pin tidak berlaku dan versi
sistem bisa berbeda.

Repo: <https://github.com/nusapeta/place> (private). Clone, issue, commit,
dan push memakai GitHub CLI. Alur lengkap ada di
[CONTRIBUTING.md](CONTRIBUTING.md#repo-dan-github-cli).

## Masalah

Aplikasi yang butuh pencarian tempat di Indonesia biasanya memakai layanan
hosted dengan biaya per request, atau menjalankan Nominatim lengkap yang
memerlukan PostgreSQL, beberapa GB RAM, dan proses impor panjang. Untuk
kebutuhan yang sempit, yaitu mencari nama tempat, POI, jalan, dan mencari
fitur terdekat, sebagian besar kapasitas itu tidak terpakai.

`nusapeta-place` mengambil batas masalah yang lebih kecil:

- Data Indonesia saja pada v1.
- Nama tempat, POI, jalan, dan wilayah administratif.
- Forward search, reverse nearest, dan lookup ID.
- Rebuild penuh dari file PBF, bukan update diff.
- Satu file index yang bisa dipromosikan dan di-rollback sebagai artefak.

## Korpus yang diukur

Sensus berikut dijalankan pada 2026-11-13 terhadap extract Indonesia yang
dipakai routeid, `data/indonesia-latest.osm.pbf`. Angka ini adalah isi korpus,
bukan hasil index `nusapeta-place`.

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
nusa-place-build  (Rust, batch)
    |
    v
places.idx + places.idx.json
    |
    v
nusa-place  (Rust, HTTP, mmap read-only)
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
| [docs/research/P4-EVIDENCE.md](docs/research/P4-EVIDENCE.md) | Relevance, reverse, beban, dan drill release |
| [docs/research/NFR-REVIEW.md](docs/research/NFR-REVIEW.md) | Status gate NFR |
| [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md) | Lisensi, audit, dan atribusi |
| [deploy/README.md](deploy/README.md) | Deploy container, Compose homelab, tunnel, update, rollback |
| [docs/releases/0.1.0.md](docs/releases/0.1.0.md) | Catatan rilis |
| [docs/GLOSSARY.md](docs/GLOSSARY.md) | Istilah teknis yang dipakai di repo |
| [docs/decisions/](docs/decisions/) | Architecture Decision Records |

Indeks dokumentasi dan aturan perubahan ada di
[docs/README.md](docs/README.md).

## Lisensi

Kode: MIT, lihat [LICENSE](LICENSE). Data OpenStreetMap: ODbL 1.0. Respons API
dan dokumentasi wajib membawa atribusi ke OpenStreetMap contributors.
