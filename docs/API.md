# API placeid

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
| GET | `/reverse` | Fitur terdekat dari koordinat |
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

## 5. `GET /reverse`

### 5.1 Parameter

| Parameter | Wajib | Rentang | Default | Catatan |
|---|---|---|---|---|
| `lat` | Ya | -90..90 | | Lintang |
| `lon` | Ya | -180..180 | | Bujur |
| `radius` | Tidak | 1 sampai 10000 meter | 1000 | Radius pencarian |
| `addressdetails` | Tidak | `0`, `1` | `0` | Sama dengan search |

`zoom` dan parameter geometri tidak didukung di v1.

### 5.2 Perilaku

- Mengembalikan satu objek fitur terdekat di dalam radius.
- Jika tidak ada fitur, mengembalikan 404 dengan `error=not_found`.
- Jarak dihitung dengan haversine. Prioritas jenis hanya dipakai saat selisih
  jarak kurang dari 25 meter.
- Tidak ada interpolasi nomor rumah.

### 5.3 Contoh

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

## 6. `GET /lookup`

### 6.1 Parameter

| Parameter | Wajib | Rentang | Catatan |
|---|---|---|---|
| `osm_ids` | Salah satu | Maksimum 50 ID | Contoh `N123,W456,R789` |
| `place_ids` | Salah satu | Maksimum 50 ID | Ekstensi, daftar angka |
| `addressdetails` | Tidak | `0`, `1` | Sama dengan search |

Hanya satu dari `osm_ids` atau `place_ids` yang boleh dikirim. Jika keduanya
dikirim, server mengembalikan 400.

### 6.2 Perilaku

- Hasil berupa array, hanya ID yang ditemukan.
- Urutan hasil mengikuti urutan permintaan.
- ID yang tidak ditemukan dihilangkan, bukan menghasilkan error.
- Awalan `N`, `W`, `R` tidak peka huruf besar/kecil.

### 6.3 Contoh

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

## 7. Endpoint operasional

### 7.1 `/healthz`

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

### 7.2 `/version`

```json
{
  "server": "placeid-serve 0.1.0",
  "api": "v1",
  "format_version": 1,
  "builder": "placeid-build 0.1.0",
  "pbf_sha256": "def456...",
  "index_sha256": "abc123...",
  "built_at": "2026-11-13T00:00:00Z"
}
```

### 7.3 `/metrics`

Format teks Prometheus. Metrik minimum:

| Metrik | Tipe | Label |
|---|---|---|
| `placeid_requests_total` | counter | `endpoint`, `status` |
| `placeid_request_duration_seconds` | histogram | `endpoint` |
| `placeid_results_total` | counter | `endpoint` |
| `placeid_index_size_bytes` | gauge | |
| `placeid_index_features` | gauge | |

Tidak ada label `q`, `lat`, `lon`, atau `place_id`.

## 8. Error

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

## 9. Kompatibilitas Nominatim

| Fitur Nominatim | placeid v1 |
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

Field respons yang tidak ada di `placeid` tidak boleh dianggap kosong secara
implisit. Klien harus memeriksa field yang dipakai.

## 10. Atribusi dan lisensi

Setiap hasil memuat:

```text
licence: "ODbL 1.0, © OpenStreetMap contributors"
```

Atribusi juga wajib terlihat di dokumentasi dan UI yang menampilkan hasil.
Data OSM berlisensi ODbL 1.0. Kode `placeid` berlisensi MIT.

## 11. Batas dan versi

- Panjang `q` maksimum 256 byte.
- `limit` maksimum 20.
- `radius` maksimum 10 km.
- `osm_ids` atau `place_ids` maksimum 50.
- Tidak ada pagination.
- Perubahan aditif pada respons tidak menaikkan versi API.
- Perubahan breaking memerlukan path baru dan masa deprekasi.
- `/version` menyebut versi API dan format index.
