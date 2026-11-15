# ADR 0004: Subset API Nominatim

| Field | Value |
|---|---|
| Status | Accepted |
| Tanggal | 2026-11-13 |
| Konteks | Nusapeta Place v1 |

## Konteks

Klien geocoding yang sudah ada biasanya memakai Nominatim. Membuat API baru
dari nol berarti setiap klien menulis adapter. Meniru Nominatim sepenuhnya
berarti mengejar parameter, format, dan perilaku yang tidak dibutuhkan v1.

## Keputusan

`nusapeta-place` menyediakan subset Nominatim `jsonv2`:

- `GET /search`
- `GET /reverse`
- `GET /lookup`
- `jsonv2` sebagai satu-satunya format
- `addressdetails`, `accept-language`, `limit`, `lat`, `lon`, `viewbox`,
  `bounded`
- `licence`, `place_id`, `osm_type`, `osm_id`, `lat`, `lon`, `class`, `type`,
  `name`, `display_name`, `importance`, `boundingbox`

Parameter yang dikenal tetapi tidak didukung menghasilkan 400 dengan
`error=unsupported_parameter`. Parameter asing diabaikan agar toleran.
Endpoint `/status` tidak disediakan; gunakan `/healthz` dan `/version`.

## Konsekuensi

Positif:

- Klien dengan field umum bisa pindah dengan perubahan kecil.
- Kontrak lebih kecil dan bisa diuji.
- Tidak ada janji parity yang tidak bisa dipenuhi.

Negatif:

- Bukan drop-in replacement penuh.
- Klien yang memakai `polygon_geojson`, `extratags`, `namedetails`, atau
  `exclude_place_ids` harus menyesuaikan.
- `place_id` berbeda dari Nominatim, jadi migrasi harus memakai `osm_type` dan
  `osm_id`.
- Perlu dokumentasi perbedaan yang jelas.

## Alternatif yang ditolak

| Alternatif | Alasan |
|---|---|
| Parity Nominatim penuh | Scope terlalu besar, banyak fitur tidak dipakai |
| API custom | Semua klien perlu adapter baru |
| GraphQL | Tidak cocok dengan klien Nominatim dan menambah kompleksitas |
| Proxy ke Nominatim | Tidak menyelesaikan target footprint dan operasi |

## Tinjau ulang

Jika ada klien yang benar-benar membutuhkan parameter yang ditolak, tambahkan
satu per satu dengan test kontrak. Jangan menambah parameter karena "Nominatim
punya" tanpa pengguna yang jelas.
