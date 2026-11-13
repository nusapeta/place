# Bukti P3: kualitas index

Semua angka di halaman ini diukur pada mesin AMD Ryzen 5 5600G, 12 thread,
14 GiB RAM, Linux 7.2.15-1-omarchy x64, commit kerja sebelum P4. Input
`indonesia-latest.osm.pbf` berukuran 1.741.650.872 B. Angka adalah hasil
pengukuran, bukan estimasi, kecuali yang ditandai tegas.

## Ringkasan build penuh

| Metrik | Nilai |
|---|---:|
| Wall build | 231,29 s |
| Peak RSS builder | 8.167.108 KiB |
| Ukuran index | 141.997.032 B |
| SHA-256 file index | `a732b4ecaa7c1e9e36f7b475ab891c65d1f485744cc3cbdb9da69eac4d8b836d` |
| SHA-256 body di manifest | `0df44207907a007459f53285ad727a31441138456bd5f017d408082d00c62bca` |
| Feature | 1.226.755 |
| Admin | 9.360 |
| Name | 1.374.327 |
| Posting | 4.837.518 |
| Feature dilewati | 50.179.543 |
| Polygon admin gagal | 43 |
| Node ref hilang | 46 |

Fase build penuh: `features=54,20 s`, `way_geometry=46,23 s`,
`node_coordinates=54,42 s`, `assemble=45,90 s`. Sisa waktu adalah tulis,
checksum, dan verifikasi.

## P3-01 Hanya fitur bernama

Test `skips_features_without_a_name` membangun fixture dengan
`addr:housenumber` tanpa `name` dan memastikan fitur tidak masuk index. Build
penuh melewati 50.179.543 fitur tanpa nama atau tanpa geometri yang bisa
dipakai.

## P3-02 Node ref hilang

Builder menghitung referensi node yang hilang. Manifest menyimpan field
`missing_node_refs` dengan `#[serde(default)]` agar manifest lama tetap bisa
dibaca. Build penuh mencatat 46 referensi hilang dan menulis peringatan.
Test membaca manifest lama tanpa field dan memastikan nilainya 0.

## P3-03 Polygon admin rusak

Build penuh mencatat 43 polygon admin yang gagal dirakit dan tetap
menyelesaikan build. Fitur yang membutuhkan polygon tersebut tidak mendapat
admin parent; fitur lain tidak terpengaruh. Peringatan ditulis ke stderr,
bukan error fatal.

## P3-04 Hierarki admin

Test `assigns_deepest_smallest_parent` memastikan fitur memilih parent
terdalam dengan luas terkecil ketika beberapa polygon memuat titik yang sama.
Sebaran level admin pada index penuh:

| Level | Jumlah |
|---:|---:|
| 4 | 50 |
| 5 | 583 |
| 6 | 2.180 |
| 7 | 6.547 |

Level lain tidak muncul pada data Indonesia yang dipakai.

## P3-05 Varian nama dan brand

Builder mengindeks `name`, `name:id`, `name:en`, `alt_name`, `old_name`,
`official_name`, `short_name`, dan `brand`. Test
`indexes_declared_variants_and_brand` memastikan varian bahasa dan brand
masuk FST serta output mempertahankan nama asli. Index penuh memiliki
1.374.327 baris nama dari 1.226.755 fitur. Selisih 147.572 baris berasal dari
varian bahasa, alt name, dan brand.

## P3-06 Importance dari tag OSM

Test `importance_follows_feature_class` memastikan admin level 4 sampai 7
lebih tinggi dari place, place lebih tinggi dari highway, highway lebih tinggi
dari POI, dan POI lebih tinggi dari building pada kondisi tag lain sama.
Keputusan lengkap ada di [ADR 0007](../decisions/0007-importance.md).

## P3-07 Determinisme

Dua build penuh berurutan menghasilkan byte index identik:

```text
a30ebe9c2eec9c445c4cbfb94763504b92df8bf5a8df55c814f32e2c70750bc0  /tmp/placeid-full/places.idx
a30ebe9c2eec9c445c4cbfb94763504b92df8bf5a8df55c814f32e2c70750bc0  /tmp/placeid-full2/places.idx
```

Manifest setelah `built_at` dihapus juga identik:
`dd11d829ed160ad53ade6536aa59343e7ac65d7a9ccccc1f34cf649ed7f639be`
untuk kedua file. Build ketiga setelah penambahan brand menghasilkan body hash
manifest `0df442...` karena isi index berubah, bukan karena nondeterminisme.
Build keempat dari kode final menghasilkan file yang identik byte demi byte
dengan build ketiga: file SHA-256 `a732b4ec...`, body manifest `0df442079...`.
Selisih kedua nilai berasal dari trailer checksum yang tidak ikut dihitung
sebagai body; `places.idx.sha256` pada rilis memakai hash file penuh,
sedangkan manifest memakai hash body.

## P3-08 Waktu dan ukuran

Waktu dan ukuran ada di tabel ringkasan. Build memakai 12 worker dan seluruh
fase berurutan; tidak ada pekerjaan yang dijalankan paralel di luar fase
tersebut. Nilai ini adalah pengukuran satu run pada page cache hangat.

## P3-09 Smoke index penuh

`python3 scripts/smoke_index.py` pada server index penuh:

- 20 query maju mengembalikan 5 hasil dengan `boundingbox` valid.
- 10 query balik mengembalikan 200 dengan koordinat di dalam radius.
- 0 respons kosong, 0 error HTTP.

Startup diukur 5 kali pada page cache hangat:

| Run | Startup | VmRSS setelah 1 s |
|---:|---:|---:|
| 1 | 360,99 ms | 6.140 KiB |
| 2 | 324,71 ms | 6.196 KiB |
| 3 | 337,91 ms | 6.256 KiB |
| 4 | 325,07 ms | 6.276 KiB |
| 5 | 329,48 ms | 6.164 KiB |

p50 329,48 ms dan maksimum 360,99 ms. Semua di bawah NFR-005 1 detik.
Pengukuran ini belum memakai page cache dingin karena mesin tidak memberi
akses drop cache; angka dingin adalah batas atas yang belum diverifikasi.
