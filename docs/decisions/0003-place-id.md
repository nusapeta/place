# ADR 0003: place_id 53-bit

| Field | Value |
|---|---|
| Status | Accepted |
| Tanggal | 2026-11-13 |
| Konteks | Nusapeta Place v1 |

## Konteks

`place_id` dipakai klien untuk menyimpan tempat dan membukanya kembali. Kalau
ID dibuat sebagai nomor urut saat build, ID berubah setiap rebuild. Itu tidak
cocok dengan nama produk dan membuat cache klien rapuh.

Nominatim memakai `place_id` numerik yang stabil dalam satu database, tetapi
tidak menjamin arti yang sama setelah impor ulang. `nusapeta-place` butuh aturan yang
bisa dihitung ulang dari data OSM.

Kendala lain: banyak klien memparse JSON dengan JavaScript. Angka di atas
`2^53 - 1` kehilangan presisi. Jika ID memakai 64 bit penuh, klien JavaScript
akan mendapat nilai yang salah.

## Keputusan

`place_id` diturunkan dari tipe dan ID OSM:

```text
place_id = (type_code << 51) | osm_id

type_code:
  node     = 1
  way      = 2
  relation = 3
```

Syarat:

- `osm_id < 2^51`.
- `place_id` selalu ≤ `2^53 - 1`, sehingga aman sebagai angka JavaScript.
- `place_id` dikirim sebagai angka, bukan string.
- `place_id` unik selama `(osm_type, osm_id)` unik.
- `place_id` tidak berubah antar rebuild untuk objek OSM yang sama.

Type code 0 tidak dipakai. Objek sintetis tanpa OSM ID tidak dibuat di v1.
Admin hierarchy memakai `admin_id` internal, bukan `place_id` sintetis.

## Konsekuensi

Positif:

- ID bisa dihitung ulang tanpa tabel mapping.
- Tidak ada nomor urut yang berubah saat rebuild.
- Aman untuk JavaScript dan JSON number.
- Lookup bisa memakai binary search karena places diurutkan `place_id`.

Negatif:

- ID bergantung pada OSM ID. Jika objek dihapus dan dibuat ulang dengan ID
  baru, ID lama tidak berlaku.
- Batas `2^51` harus divalidasi. Saat ini ID OSM jauh di bawah batas, tetapi
  builder tetap harus menolak overflow.
- `place_id` bukan UUID dan tidak bisa dipakai sebagai identitas abadi di luar
  keberadaan objek OSM.
- Dua objek OSM berbeda yang merujuk tempat yang sama tetap punya ID berbeda.
  Dedup lintas tipe bukan bagian v1.

## Alternatif yang ditolak

| Alternatif | Alasan |
|---|---|
| Nomor urut build | Berubah setiap rebuild, cache klien rusak |
| UUID v4 | Tidak bisa dihitung ulang dari OSM, butuh tabel mapping |
| String gabungan `type/id` | Tidak kompatibel dengan field `place_id` numerik Nominatim |
| 64-bit penuh | Tidak aman untuk JavaScript |
| Hash nama dan koordinat | Tidak stabil saat nama atau geometri berubah, rawan tabrakan |

## Catatan kompatibilitas

Nominatim punya `place_id` sendiri. Klien yang membawa `place_id` dari
Nominatim tidak bisa memakainya di `nusapeta-place`. Gunakan `osm_type` dan `osm_id`
untuk migrasi, lalu simpan `place_id` baru. Perbedaan ini harus disebut di
dokumentasi API.
