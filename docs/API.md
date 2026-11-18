# API Nusapeta Place

| Field | Value |
|---|---|
| Status | Draft |
| Versi kontrak | v1 |
| Format | `jsonv2` |
| Base URL | `http://host:8090` di jaringan internal |
| Dokumen terkait | [SRS](SRS.md), [TRD](TRD.md) |

## 1. Prinsip

API ini adalah subset Nominatim `jsonv2`, bukan implementasi penuh. Klien yang
memakai field umum seperti `place_id`, `osm_type`, `osm_id`, `lat`, `lon`,
`display_name`, `address`, dan `boundingbox` bisa pindah dengan perubahan
kecil. Parameter yang tidak didukung harus terlihat jelas, bukan diam-diam
diabaikan.

Semua respons sukses memakai `Content-Type: application/json; charset=utf-8`.
Semua koordinat dikirim sebagai string desimal. `place_id` dan `osm_id`
dikirim sebagai angka.

## 2. Endpoint

| Method | Path | Fungsi |
|---|---|---|
| GET | `/search` | Forward search berdasarkan nama |
| GET | `/place/suggest` | Autocomplete prefix dari name/token FST |
| GET | `/reverse` | Fitur terdekat dari koordinat |
| GET | `/place/admin/reverse` | Admin area terdalam dari koordinat |
| GET | `/lookup` | Ambil fitur berdasarkan ID |
| GET | `/healthz` | Status index dan kesiapan server |
| GET | `/version` | Versi server dan manifest |
| GET | `/metrics` | Metrik Prometheus |

Tidak ada endpoint tulis. Method selain GET menghasilkan 405 dengan header
`Allow: GET`.

## 3. Parameter umum

| Parameter | Nilai | Default | Catatan |
|---|---|---|---|
| `format` | `jsonv2` | `jsonv2` | Format lain menghasilkan 400 |
| `addressdetails` | `0`, `1` | `0` | `1` menambahkan objek `address` |
| `accept-language` | Header, contoh `id,en` | `id` | Memilih varian nama bila ada |

Parameter yang tidak dikenal diabaikan agar toleran terhadap klien Nominatim.
Parameter yang dikenal tetapi tidak didukung menghasilkan 400 dengan
`error=unsupported_parameter`. Contoh: `polygon_geojson`, `extratags`,
`namedetails`.

## 4. `GET /search`

### 4.1 Parameter

| Parameter | Wajib | Rentang | Default | Catatan |
|---|---|---|---|---|
| `q` | Ya | 1 sampai 256 byte | | Teks pencarian |
| `limit` | Tidak | 1 sampai 20 | 5 | Jumlah hasil maksimum |
| `lat`, `lon` | Tidak | -90..90, -180..180 | | Harus dikirim berpasangan |
| `viewbox` | Tidak | `min_lon,max_lat,max_lon,min_lat` | | Bias area |
| `bounded` | Tidak | `0`, `1` | `0` | `1` wajib bersama `viewbox` |
| `countrycodes` | Tidak | `id` | `id` | Nilai lain menghasilkan 400 |

`viewbox` memakai urutan Nominatim: kiri, atas, kanan, bawah. Contoh:
`110.35,-7.78,110.40,-7.82`.

### 4.2 Perilaku

- `q` kosong atau hanya spasi menghasilkan 400.
- Hasil diurutkan berdasarkan skor. Tie-break memakai `importance` menurun,
  lalu `place_id` menaik.
- `limit` memotong hasil setelah ranking. Tidak ada pagination di v1.
- `lat`/`lon` memberi bias jarak, bukan filter. `bounded=1` memfilter dengan
  `viewbox`.
- `display_name` diakhiri `Indonesia` untuk index Indonesia.
- Query yang tidak menghasilkan kandidat mengembalikan array kosong `[]`.

### 4.3 Contoh

```http
GET /search?q=malioboro&lat=-7.7956&lon=110.3695&limit=2&addressdetails=1
```

```json
[
  {
    "place_id": 4503599627493952,
    "licence": "ODbL 1.0, © OpenStreetMap contributors",
    "osm_type": "way",
    "osm_id": 123456,
    "lat": "-7.7926000",
    "lon": "110.3658000",
    "class": "highway",
    "type": "pedestrian",
    "name": "Jalan Malioboro",
    "display_name": "Jalan Malioboro, Yogyakarta, Daerah Istimewa Yogyakarta, Indonesia",
    "importance": 0.82,
    "boundingbox": ["-7.8000", "-7.7850", "110.3600", "110.3720"],
    "address": {
      "road": "Jalan Malioboro",
      "city": "Yogyakarta",
      "state": "Daerah Istimewa Yogyakarta",
      "country": "Indonesia",
      "country_code": "id"
    }
  }
]
```

Contoh di atas ilustratif. `place_id`, `osm_id`, koordinat, dan `importance`
bukan data hasil build.


## 5. `GET /place/suggest`

### 5.1 Parameter

| Parameter | Wajib | Rentang | Default | Catatan |
|---|---|---|---|---|
| `q` | Ya | Minimal 3 karakter setelah normalisasi, maksimum 256 byte | | Awal nama yang dicari |
| `limit` | Tidak | 1 sampai 20 | 5 | Jumlah hasil maksimum |
| `lat`, `lon` | Tidak | -90..90, -180..180 | | Harus dikirim berpasangan; bias grid spasial |

### 5.2 Perilaku

- `q` dinormalisasi lebih dulu: lowercase, NFKC, diakritik dibuang, dan
  singkatan seperti `Jl.`, `Kec.`, `Kab.` diperluas.
- Kandidat diambil dari prefix walk pada name FST dan token FST yang sudah
  ada di index. Endpoint ini tidak menambah section index.
- Prefix walk token FST dibatasi 2.000 baris agar biaya tetap terkendali.
- `lat`/`lon` hanya bias, bukan filter. Sel grid yang sama atau bersebelahan
  dengan titik bias mendapat bobot tambahan sesuai jarak.
- Setiap hasil memuat `match` dengan nilai `prefix` dan `score` angka. `score`
  dipakai untuk urutan dan tidak dijamin stabil antar build index.
- Hasil urut berdasarkan `score` menurun, lalu `importance` menurun, lalu
  `place_id` naik. Query tanpa kandidat mengembalikan `[]`.

### 5.3 Contoh

```http
GET /place/suggest?q=mona&lat=-6.175&lon=106.827&limit=3
```

```json
[
  {
    "place_id": 2251799813685399,
    "name": "Monumen Nasional",
    "display_name": "Monumen Nasional, Indonesia",
    "match": "prefix",
    "score": 2.4
  }
]
```

Contoh dipangkas; respons lengkap memakai field `jsonv2` yang sama seperti
`/search`. Nilai di atas ilustratif, bukan hasil build.

## 6. `GET /reverse`

### 6.1 Parameter

| Parameter | Wajib | Rentang | Default | Catatan |
|---|---|---|---|---|
| `lat` | Ya | -90..90 | | Lintang |
| `lon` | Ya | -180..180 | | Bujur |
| `radius` | Tidak | 1 sampai 10000 meter | 1000 | Radius pencarian |
| `addressdetails` | Tidak | `0`, `1` | `0` | Sama dengan search |

`zoom` dan parameter geometri tidak didukung di v1.

### 6.2 Perilaku

- Mengembalikan satu objek fitur terdekat di dalam radius.
- Jika tidak ada fitur, mengembalikan 404 dengan `error=not_found`.
- Jarak dihitung dengan haversine. Prioritas jenis hanya dipakai saat selisih
  jarak kurang dari 25 meter.
- Tidak ada interpolasi nomor rumah.

### 6.3 Contoh

```http
GET /reverse?lat=-7.7956&lon=110.3695&radius=500&addressdetails=1
```

```json
{
  "place_id": 4503599627493952,
  "licence": "ODbL 1.0, © OpenStreetMap contributors",
  "osm_type": "way",
  "osm_id": 123456,
  "lat": "-7.7926000",
  "lon": "110.3658000",
  "class": "highway",
  "type": "pedestrian",
  "name": "Jalan Malioboro",
  "display_name": "Jalan Malioboro, Yogyakarta, Daerah Istimewa Yogyakarta, Indonesia",
  "importance": 0.82,
  "boundingbox": ["-7.8000", "-7.7850", "110.3600", "110.3720"],
  "address": {
    "road": "Jalan Malioboro",
    "city": "Yogyakarta",
    "state": "Daerah Istimewa Yogyakarta",
    "country": "Indonesia",
    "country_code": "id"
  }
}
```

## 7. `GET /place/admin/reverse`

### 7.1 Parameter

| Parameter | Wajib | Rentang | Default | Catatan |
|---|---|---|---|---|
| `lat` | Ya | -90..90 | | Lintang |
| `lon` | Ya | -180..180 | | Bujur |

Endpoint ini selalu mengembalikan objek `address`. Parameter
`addressdetails` tidak mengubah respons.

### 7.2 Perilaku

- Mengembalikan admin area terdalam yang polygonnya mengandung titik.
- Hierarchy diambil dari `parent_id` admin, bukan dari jarak ke centroid.
- `display_name` berisi nama admin dari terdalam ke provinsi lalu `Indonesia`.
- `address` memakai `village` untuk level 7, `county` untuk level 6, `city`
  untuk level 5, dan `state` untuk level 4.
- `ambiguous` bernilai `true` saat lebih dari satu polygon pada level terdalam
  mengandung titik. Pilihan deterministik memakai `admin_id` terkecil.
- Polygon administrator berlaku sebagai hole bila berada setelah outer ring.
- 404 jika tidak ada polygon yang mengandung titik.
- 501 dengan `error=no_admin_polygons` jika index tidak memuat section polygon.

### 7.3 Contoh

```http
GET /place/admin/reverse?lat=-6.195&lon=106.815
```

```json
{
  "place_id": 1351079888221220,
  "licence": "ODbL 1.0, © OpenStreetMap contributors",
  "osm_type": "relation",
  "osm_id": 1003,
  "lat": "-6.1950000",
  "lon": "106.8150000",
  "class": "boundary",
  "type": "administrative",
  "name": "Gambir Village",
  "display_name": "Gambir Village, Gambir, Jakarta Pusat, Daerah Khusus Jakarta, Indonesia",
  "address": {
    "village": "Gambir Village",
    "county": "Gambir",
    "city": "Jakarta Pusat",
    "state": "Daerah Khusus Jakarta",
    "country": "Indonesia",
    "country_code": "id"
  },
  "admin_id": 4,
  "ambiguous": false
}
```

Contoh di atas memakai fixture `tests/fixtures/mini.pbf`, bukan data produksi.

## 8. `GET /lookup`

### 8.1 Parameter

| Parameter | Wajib | Rentang | Catatan |
|---|---|---|---|
| `osm_ids` | Salah satu | Maksimum 50 ID | Contoh `N123,W456,R789` |
| `place_ids` | Salah satu | Maksimum 50 ID | Ekstensi, daftar angka |
| `addressdetails` | Tidak | `0`, `1` | Sama dengan search |

Hanya satu dari `osm_ids` atau `place_ids` yang boleh dikirim. Jika keduanya
dikirim, server mengembalikan 400.

### 8.2 Perilaku

- Hasil berupa array, hanya ID yang ditemukan.
- Urutan hasil mengikuti urutan permintaan.
- ID yang tidak ditemukan dihilangkan, bukan menghasilkan error.
- Awalan `N`, `W`, `R` tidak peka huruf besar/kecil.

### 8.3 Contoh

```http
GET /lookup?osm_ids=N123,W456&addressdetails=0
```

```json
[
  {
    "place_id": 2251799813685371,
    "licence": "ODbL 1.0, © OpenStreetMap contributors",
    "osm_type": "node",
    "osm_id": 123,
    "lat": "-7.8000000",
    "lon": "110.3700000",
    "class": "place",
    "type": "village",
    "name": "Contoh",
    "display_name": "Contoh, Yogyakarta, Indonesia",
    "importance": 0.40,
    "boundingbox": ["-7.8100", "-7.7900", "110.3600", "110.3800"]
  }
]
```

## 9. Endpoint operasional

### 9.1 `/healthz`

```json
{
  "status": "ok",
  "index": "places.idx",
  "format_version": 1,
  "manifest_sha256": "abc123..."
}
```

Status 200 hanya jika index sudah termap dan header valid. Status 503 jika
index belum siap. Endpoint ini tidak melakukan pencarian.

### 9.2 `/version`

```json
{
  "server": "nusa-place 0.1.0",
  "api": "v1",
  "format_version": 1,
  "builder": "nusa-place-build 0.1.0",
  "pbf_sha256": "def456...",
  "index_sha256": "abc123...",
  "built_at": "2026-11-13T00:00:00Z"
}
```

### 9.3 `/metrics`

Format teks Prometheus. Metrik minimum:

| Metrik | Tipe | Label |
|---|---|---|
| `nusa_place_requests_total` | counter | `endpoint`, `status` |
| `nusa_place_request_duration_seconds` | histogram | `endpoint` |
| `nusa_place_results_total` | counter | `endpoint` |
| `nusa_place_index_size_bytes` | gauge | |
| `nusa_place_index_features` | gauge | |

Tidak ada label `q`, `lat`, `lon`, atau `place_id`.

## 10. Error

Format:

```json
{
  "error": "invalid_request",
  "reason": "q is required"
}
```

| Status | `error` | Kapan |
|---|---|---|
| 400 | `invalid_request` | Parameter hilang, tipe salah, di luar rentang |
| 400 | `unsupported_parameter` | Parameter dikenal tetapi tidak didukung |
| 404 | `not_found` | Reverse tidak menemukan fitur |
| 405 | `method_not_allowed` | Method selain GET |
| 413 | `payload_too_large` | Body melebihi batas |
| 429 | `rate_limited` | Dibatasi gateway |
| 503 | `index_unavailable` | Index belum siap |
| 500 | `internal_error` | Kesalahan tak terduga, tanpa detail internal |

Header `X-Request-ID` selalu dikembalikan. Untuk 429 dan 503, sertakan
`Retry-After` dalam detik.

## 11. Kompatibilitas Nominatim

| Fitur Nominatim | Nusapeta Place v1 |
|---|---|
| `format=jsonv2` | Didukung |
| `q`, `limit`, `lat`, `lon`, `viewbox`, `bounded` | Didukung dengan batas |
| `addressdetails` | Didukung |
| `accept-language` | Didukung untuk `id` dan `en` |
| `polygon_geojson` | Tidak didukung, 400 |
| `extratags`, `namedetails` | Tidak didukung, 400 |
| `exclude_place_ids` | Tidak didukung, 400 |
| `countrycodes` | Hanya `id` |
| `dedupe` | Tidak didukung, 400 |
| `place_id` | Stabil dari `(osm_type, osm_id)`, bukan `place_id` Nominatim |
| `osm_ids` | Bisa dipakai untuk migrasi dari Nominatim |
| `/status` | Tidak ada; gunakan `/healthz` dan `/version` |
| `/deletable` | Tidak ada |

Field respons yang tidak ada di `nusapeta-place` tidak boleh dianggap kosong secara
implisit. Klien harus memeriksa field yang dipakai.

## 12. Atribusi dan lisensi

Setiap hasil memuat:

```text
licence: "ODbL 1.0, © OpenStreetMap contributors"
```

Atribusi juga wajib terlihat di dokumentasi dan UI yang menampilkan hasil.
Data OSM berlisensi ODbL 1.0. Kode `nusapeta-place` berlisensi MIT.

## 13. Batas dan versi

- Panjang `q` maksimum 256 byte; `q` pada `/place/suggest` minimal 3 karakter.
- `limit` maksimum 20.
- `radius` maksimum 10 km.
- `osm_ids` atau `place_ids` maksimum 50.
- Tidak ada pagination.
- Perubahan aditif pada respons tidak menaikkan versi API.
- Perubahan breaking memerlukan path baru dan masa deprekasi.
- `/version` menyebut versi API dan format index.
