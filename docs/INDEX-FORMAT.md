# Format index Nusapeta Place

| Field | Value |
|---|---|
| Status | Draft v2 |
| Versi format | 2 |
| Endianness | Little-endian |
| File | `places.idx` |
| Manifest | `places.idx.json` |
| Dokumen terkait | [TRD](TRD.md), [SRS](SRS.md) |

Dokumen ini menetapkan kontrak file index. Layout byte boleh berubah sebelum
fase 1 selesai, tetapi field, invariant, dan aturan validasi adalah kontrak
yang harus dipertahankan. Setelah fase 1, perubahan layout menaikkan
`format_version`.

Format v2 menambah dua section opsional: polygon admin dan geometri jalan.
Reader v2 masih membaca index v1 selama `flags` bernilai 0. Index dengan
section opsional wajib memakai `format_version = 2` karena reader v1 menolak
flag yang tidak dikenal.

## 1. Prinsip

- Index immutable setelah ditulis.
- Semua offset relatif terhadap awal file, bukan awal section.
- Semua section dialign 8 byte.
- Tidak ada pointer, padding acak, atau struktur bahasa pemrograman yang
  ditulis langsung.
- Record fixed-size, string disimpan di satu blok UTF-8.
- Urutan record dan postings deterministik.
- Checksum melindungi index body, bukan manifest.

## 2. Layout file

```text
+----------------------+ 0
| Header 256 byte      |
+----------------------+
| Section table        |  bagian dari header
+----------------------+
| Places               |
+----------------------+
| Class dictionary     |
+----------------------+
| Type dictionary      |
+----------------------+
| Strings              |
+----------------------+
| Name variants        |
+----------------------+
| Token FST            |
+----------------------+
| Postings             |
+----------------------+
| Name FST             |
+----------------------+
| Grid                 |
+----------------------+
| Admin                |
+----------------------+
| Admin polygons       |  opsional, boleh length 0
+----------------------+
| Way lines             |  opsional, boleh length 0
+----------------------+
| Trailer SHA-256      |  32 byte
+----------------------+ EOF
```

## 3. Header

Header berukuran tepat 256 byte.

| Offset | Ukuran | Field | Isi |
|---:|---:|---|---|
| 0 | 8 | `magic` | `PLACEID\0` (identifier format, dipertahankan saat rename) |
| 8 | 2 | `format_version` | `1` atau `2` |
| 10 | 2 | `flags` | Bit 0: admin polygons, bit 1: way lines |
| 12 | 4 | `header_bytes` | `256` |
| 16 | 4 | `feature_count` | Jumlah place record |
| 20 | 4 | `admin_count` | Jumlah admin record |
| 24 | 4 | `class_count` | Jumlah entri class |
| 28 | 4 | `type_count` | Jumlah entri type |
| 32 | 4 | `min_lat_e7` | Bounding box index |
| 36 | 4 | `min_lon_e7` | |
| 40 | 4 | `max_lat_e7` | |
| 44 | 4 | `max_lon_e7` | |
| 48 | 8 | `string_bytes` | Ukuran section strings |
| 56 | 8 | `posting_count` | Total entri postings |
| 64 | 192 | `sections` | 12 entri, masing-masing 16 byte |

Entri section:

| Offset relatif | Ukuran | Field |
|---:|---:|---|
| 0 | 8 | `offset` |
| 8 | 8 | `len` |

Urutan section:

| Index | Section |
|---:|---|
| 0 | Places |
| 1 | Class dictionary |
| 2 | Type dictionary |
| 3 | Strings |
| 4 | Name variants |
| 5 | Token FST |
| 6 | Postings |
| 7 | Name FST |
| 8 | Grid |
| 9 | Admin |
| 10 | Admin polygons, boleh length 0 |
| 11 | Way lines, boleh length 0 |

Offset 0 pada header bukan section. Section pertama minimal 256.

## 4. Places

`feature_count` record, masing-masing 64 byte, diurutkan menaik berdasarkan
`place_id`.

| Offset | Ukuran | Field | Catatan |
|---:|---:|---|---|
| 0 | 8 | `place_id` | `(type << 51) OR osm_id` |
| 8 | 4 | `name_off` | Offset ke strings |
| 12 | 2 | `name_len` | Panjang byte UTF-8 |
| 14 | 1 | `kind` | 1 node, 2 way, 3 relation |
| 15 | 1 | `flags` | Reserved, isi 0 |
| 16 | 4 | `lat_e7` | Lintang × 10^7 |
| 20 | 4 | `lon_e7` | Bujur × 10^7 |
| 24 | 4 | `importance` | `f32`, 0 sampai 1 |
| 28 | 4 | `admin_id` | 0 bila tidak ada |
| 32 | 4 | `bbox_min_lat_e7` | |
| 36 | 4 | `bbox_min_lon_e7` | |
| 40 | 4 | `bbox_max_lat_e7` | |
| 44 | 4 | `bbox_max_lon_e7` | |
| 48 | 2 | `class_id` | Index ke class dictionary |
| 50 | 2 | `type_id` | Index ke type dictionary |
| 52 | 4 | `names_off` | Offset ke name variants |
| 56 | 1 | `names_count` | Jumlah varian, maksimum 255 |
| 57 | 7 | `reserved` | Harus 0 |

Aturan:

- `name_len` tidak boleh 0 untuk fitur yang masuk index.
- `name_off + name_len` harus di dalam section strings.
- `lat_e7` dan `lon_e7` harus dalam rentang valid setelah dibagi 10^7.
- `admin_id` harus 0 atau menunjuk admin record yang ada.
- `place_id` unik dan valid menurut ADR 0003.
- `kind` harus konsisten dengan dua bit atas `place_id`.
- `importance` harus finite dan 0 sampai 1.
- `bbox` harus berisi titik `(lat, lon)`.

## 5. Class dan type dictionary

Section dimulai dengan:

| Offset | Ukuran | Field |
|---:|---:|---|
| 0 | 4 | `count` |
| 4 | 4 | `reserved` |

Diikuti `count` entri 8 byte:

| Offset | Ukuran | Field |
|---:|---:|---|
| 0 | 4 | `str_off` |
| 4 | 2 | `str_len` |
| 6 | 2 | `pad` |

`str_off` dan `str_len` menunjuk ke strings. Entri diurutkan secara
leksikografis dan tidak boleh duplikat. `class_id` dan `type_id` adalah index
entri, dimulai dari 0.

## 6. Strings

Blok UTF-8 tanpa terminator. Offset dan panjang dipakai oleh place, admin,
class, type, dan name variants. Aturan:

- Semua string harus UTF-8 valid.
- Offset harus kurang dari `string_bytes`.
- `offset + length` tidak boleh melewati section.
- String boleh duplikat, tetapi builder **SHOULD** melakukan dedup untuk
  menghemat ruang.
- Tidak ada null byte di dalam string.

## 7. Name variants

Array entri 12 byte. `names_off` pada place atau admin menunjuk ke awal array,
`names_count` menentukan jumlah entri.

| Offset | Ukuran | Field | Catatan |
|---:|---:|---|---|
| 0 | 3 | `lang` | Kode bahasa ASCII, contoh `id`, `en` |
| 3 | 1 | `pad` | 0 |
| 4 | 4 | `str_off` | Offset ke strings |
| 8 | 2 | `str_len` | Panjang byte |
| 10 | 2 | `pad` | 0 |

Varian boleh termasuk nama utama. Varian dengan `lang` kosong tidak valid.
Urutan varian deterministik, diurutkan berdasarkan `lang` lalu string.

## 8. Token FST

Section berisi byte FST (`fst::Map`). Key adalah token ternormalisasi UTF-8.
Value 64-bit:

```text
value = (posting_offset << 32) | posting_count
```

`posting_offset` adalah index entri pertama di section postings, bukan byte
offset. `posting_count` adalah jumlah entri. Keduanya harus muat 32 bit.

Aturan:

- Key tidak boleh kosong.
- Key harus sudah dinormalisasi.
- Value harus menunjuk rentang postings yang valid.
- FST harus bisa diparse ulang saat validasi.

## 9. Postings

Array `u32` row index place, bukan `place_id`. Setiap daftar postings:

- Terurut menaik.
- Tidak berisi duplikat.
- Semua nilai kurang dari `feature_count`.

Row index adalah posisi record di section places, dimulai dari 0. Karena
places diurutkan berdasarkan `place_id`, row index stabil untuk index yang
sama.

Format v1 memakai `u32` absolut, bukan delta varint. **Measured** pada index
Indonesia: 4.834.910 row postings total; subset token FST 3.608.155 row.
Encoding absolut 14.432.620 byte, delta varint 5.767.275 byte (rasio 2,503),
dan decode varint 2,56 ns per posting. Penghematan 8,7 MB tidak sebanding
dengan kehilangan random access dan biaya decode saat startup.

## 10. Name FST

Section berisi byte FST. Key adalah nama lengkap ternormalisasi. Value adalah
row index place (64-bit). Jika beberapa place punya nama yang sama, hanya satu
yang bisa menjadi value FST. Untuk kasus itu:

- Name FST menyimpan row dengan `importance` tertinggi.
- Semua place tetap bisa ditemukan lewat token FST.
- Query nama duplikat **MUST** mengembalikan lebih dari satu kandidat jika
  `limit` cukup; ini diuji dengan `Indomaret` pada fixture.
- Name FST bukan satu-satunya jalur pencarian.

Aturan ini menghindari struktur value berulang di FST. Dedup penuh dilakukan
di ranker, bukan di FST.

## 11. Grid

Section dimulai dengan header 40 byte:

| Offset | Ukuran | Field | Catatan |
|---:|---:|---|---|
| 0 | 4 | `cell_size_e7` | Default 100000 (0,01 derajat) |
| 4 | 4 | `min_lat_e7` | Awal grid, sudah dipad |
| 8 | 4 | `min_lon_e7` | |
| 12 | 4 | `cols` | Jumlah kolom |
| 16 | 4 | `rows` | Jumlah baris |
| 20 | 4 | `cell_count` | Jumlah sel terisi (sparse) |
| 24 | 4 | `reserved` | 0 |
| 28 | 12 | `reserved` | 0 |

Diikuti `cell_count` entri 12 byte, terurut menaik berdasarkan `key`:

| Offset | Ukuran | Field |
|---:|---:|---|
| 0 | 4 | `key` |
| 4 | 4 | `posting_off` |
| 8 | 4 | `posting_count` |

`key = row * cols + col`. Hanya sel terisi yang disimpan; sel kosong tidak
punya entri. `posting_off` menunjuk ke section postings, `posting_count`
adalah jumlah row index. Row di setiap sel terurut dan unik. Lookup memakai
binary search pada `key`.

Ukuran section adalah `40 + 12 * cell_count` byte. Dense grid
`cols * rows` ditolak karena untuk Indonesia jumlah selnya jauh lebih besar
daripada jumlah fitur. **Measured** pada index Indonesia penuh: 187.872 sel
terisi dari 14.121.147 sel dense, sehingga section grid 2.254.504 byte.
Dense 0,01 derajat pada bbox yang sama membutuhkan 169,5 MB hanya untuk tabel
sel. Radius 1 km menyentuh p95 9 sel dan 2.969 kandidat; radius 10 km
menyentuh p95 380 sel dan 89.443 kandidat sebelum cap kandidat server.

Grid memakai koordinat `lat_e7` dan `lon_e7` untuk menghitung sel:

```text
col = (lon_e7 - min_lon_e7) / cell_size_e7
row = (lat_e7 - min_lat_e7) / cell_size_e7
```

Bounding box index harus tercakup grid. Jika tidak, index tidak valid.

## 12. Admin

Section dimulai dengan `admin_count` record 40 byte. Record diurutkan menaik
berdasarkan `admin_id`.

| Offset | Ukuran | Field | Catatan |
|---:|---:|---|---|
| 0 | 4 | `admin_id` | Unik, bukan 0 |
| 4 | 4 | `parent_id` | 0 bila tidak ada |
| 8 | 1 | `level` | `admin_level` OSM |
| 9 | 1 | `flags` | Reserved |
| 10 | 2 | `reserved` | 0 |
| 12 | 4 | `name_off` | |
| 16 | 2 | `name_len` | |
| 18 | 4 | `names_off` | |
| 22 | 1 | `names_count` | |
| 23 | 1 | `reserved` | 0 |
| 24 | 4 | `bbox_min_lat_e7` | |
| 28 | 4 | `bbox_min_lon_e7` | |
| 32 | 4 | `bbox_max_lat_e7` | |
| 36 | 4 | `bbox_max_lon_e7` | |

Aturan:

- `parent_id` harus 0 atau menunjuk admin lain.
- Rantai parent tidak boleh memiliki siklus.
- `level` harus 4 sampai 7 untuk v1.
- `admin_id` tidak boleh 0.
- Nama admin harus ada di strings.

## 13. Admin polygons

Section opsional. Jika `flags` bit 0 tidak diset, `len` harus 0. Endpoint
`/place/admin/reverse` memakai section ini untuk point-in-polygon.

Header:

| Offset | Ukuran | Field |
|---:|---:|---|
| 0 | 4 | `polygon_count` |
| 4 | 4 | `reserved` |

Directory `polygon_count` entri 20 byte, diurutkan menaik berdasarkan
`admin_id`:

| Offset | Ukuran | Field |
|---:|---:|---|
| 0 | 4 | `admin_id` |
| 4 | 8 | `place_id` |
| 12 | 4 | `offset` |
| 16 | 4 | `len` |

`offset` relatif terhadap akhir directory. Payload tiap polygon:

| Ukuran | Field |
|---:|---|
| 4 | `ring_count` |
| 4 | `point_count` per ring, dalam `i32`, genap, minimal 6 |
| `point_count * 4` | pasangan `lat_e7`, `lon_e7` berurutan |

Aturan:

- `admin_id` unik, antara 1 dan `admin_count`.
- `place_id` adalah ID relasi boundary yang valid.
- `offset + len` tidak boleh melewati section.
- `len` harus sama dengan ukuran payload yang diparse.

## 14. Way lines

Section opsional. Jika `flags` bit 1 tidak diset, `len` harus 0. Endpoint
`/reverse` memakai section ini untuk jarak ke geometri jalan.

Header:

| Offset | Ukuran | Field |
|---:|---:|---|
| 0 | 4 | `way_count` |
| 4 | 4 | `reserved` |

Directory `way_count` entri 12 byte, diurutkan menaik berdasarkan row place:

| Offset | Ukuran | Field |
|---:|---:|---|
| 0 | 4 | `row` |
| 4 | 4 | `offset` |
| 8 | 4 | `points` |

`offset` relatif terhadap akhir directory. Payload adalah `points` nilai
`i32` berurutan: pasangan `lat_e7`, `lon_e7`. `points` genap dan minimal 4.

## 15. Trailer

32 byte terakhir adalah SHA-256 dari semua byte sebelum trailer. Trailer tidak
termasuk section table. Manifest memuat checksum yang sama.

Server **MUST** memvalidasi trailer jika `--verify-checksum` diberikan.
Tanpa flag, server **MUST** memvalidasi header, section bounds, dan trailer
length, tetapi **MAY** melewati hashing penuh.

## 16. Manifest

Manifest adalah JSON terpisah, ditulis setelah index selesai. Field minimum:

```json
{
  "format_version": 2,
  "builder": "nusa-place-build 0.2.0",
  "built_at": "2026-11-13T00:00:00Z",
  "pbf": {
    "name": "indonesia-latest.osm.pbf",
    "sha256": "...",
    "bytes": 0
  },
  "index": {
    "name": "places.idx",
    "sha256": "...",
    "bytes": 0
  },
  "counts": {
    "features": 0,
    "admins": 0,
    "names": 0,
    "postings": 0,
    "skipped_features": 0,
    "invalid_polygons": 0
  },
  "bbox": [0.0, 0.0, 0.0, 0.0],
  "warnings": []
}
```

`built_at` boleh berbeda antar build. `index.sha256` adalah checksum index
body, sama dengan nilai di trailer. `index.bytes` adalah ukuran total file
termasuk trailer 32 byte. `index.sha256` harus sama untuk PBF dan builder yang
sama.

## 17. Validasi

Server **MUST** memeriksa:

1. Magic dan `format_version`.
2. `header_bytes` minimal 256.
3. Semua section offset dan length berada di dalam file, tidak overlap, dan
   dialign 8 byte.
4. `sections[10].len == 0` jika dan hanya jika bit 0 tidak diset.
5. `sections[11].len == 0` jika dan hanya jika bit 1 tidak diset.
6. Isi polygon dan way lines valid menurut section 13 dan 14.
7. `feature_count`, `admin_count`, `class_count`, dan `type_count` masuk akal
   terhadap ukuran section.
8. Record places terurut dan `place_id` unik.
9. Semua offset string dan name variant berada di dalam section.
10. Semua string UTF-8 valid.
11. Semua posting row kurang dari `feature_count`, terurut, dan unik.
12. FST bisa diparse.
13. Grid menutupi bounding box index.
14. Parent admin ada dan tidak ada siklus.
15. Trailer 32 byte dan, bila diminta, checksum cocok.

Jika salah satu gagal, server **MUST** menolak index dan exit non-zero.

## 18. Versioning dan determinisme

- Perubahan layout, field, atau arti nilai menaikkan `format_version`.
- Reader v2 **MUST** menerima v1 hanya bila `flags == 0`.
- Index dengan section opsional **MUST** memakai `format_version = 2`.
- Reader **MUST** menolak versi mayor yang tidak dikenal.
- Penambahan field di akhir record boleh dilakukan dengan menaikkan versi.
- Builder **MUST** mengurutkan places, postings, strings dictionary, dan admin
  secara eksplisit.
- Builder **MUST NOT** memakai urutan iterasi hash map.
- Index body **MUST** identik untuk PBF dan builder yang sama.
- Manifest **MAY** berbeda pada `built_at` dan path input.
