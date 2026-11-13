# Profil korpus Indonesia

| Field | Value |
|---|---|
| Status | Measured |
| Tanggal | 2026-11-13 |
| Sumber | `indonesia-latest.osm.pbf`, 1.741.650.872 byte |
| Alat | pyosmium 4.x, Python 3.14.8, 24 detik |
| Data | [corpus-profile.json](corpus-profile.json) |

Semua angka di halaman ini adalah **measured** pada PBF di atas, bukan
estimasi. Berkas JSON hanya berisi agregat: tidak ada dump node, way, atau
relation yang disalin ke repo.

## 1. Fitur bernama

| Kategori | Jumlah |
|---|---:|
| Node | 346.846 |
| Way | 878.076 |
| Relation | 39.027 |
| Total | 1.263.949 |

Total ini sama dengan angka README, sehingga selisihnya 0,00%. Fitur tanpa
`name` tidak dihitung.

## 2. Nama dan token

| Metrik | Nilai |
|---|---:|
| Nama mentah unik | 645.238 |
| Nama ternormalisasi unik | 612.877 |
| Nama duplikat setelah normalisasi | 129.914 |
| Token occurrences | 3.453.904 |
| Token unik | 184.938 |
| Total karakter token | 1.323.890 |
| Byte nama mentah | 21.243.045 |
| Byte nama ternormalisasi | 20.934.951 |
| Varian bahasa dan alias | 118.348 |
| Byte teks varian | 1.972.346 |

Normalisasi menurunkan nama unik sebesar 32.361 (5,0%). Karena itu name FST
tidak boleh dijadikan satu-satunya jalur recall: 129.914 nama menunjuk lebih
dari satu fitur.

## 3. Nama dan token teratas

| Nama | Fitur |
|---|---:|
| `indomaret` | 4.141 |
| `alfamart` | 3.591 |
| `linha de agua` | 2.192 |
| `toko` | 1.964 |
| `blok` | 1.875 |
| `telkomsel` | 1.546 |
| `pertamina` | 1.526 |
| `gudang` | 1.482 |
| `bri` | 1.341 |
| `jalan jenderal sudirman` | 1.336 |

| Token | Occurrences |
|---|---:|
| `jalan` | 430.290 |
| `gang` | 82.445 |
| `raya` | 39.776 |
| `masjid` | 36.583 |
| `1` | 25.242 |
| `ii` | 24.244 |
| `al` | 23.904 |
| `desa` | 23.348 |
| `kantor` | 23.070 |
| `2` | 22.502 |

Token `jalan` muncul 430.290 kali. Query umum ini wajib dibatasi oleh
candidate cap dan tidak boleh mengalokasikan seluruh postings.

## 4. Occupancy grid

| Ukuran sel | Sel terisi | Entri sparse (12 byte) | Sel dense pada bbox |
|---|---:|---:|---:|
| 0,01 derajat | 128.669 | 1.544.028 byte | 7.883.100 |
| 0,02 derajat | 82.515 | 990.180 byte | 1.970.775 |
| 0,05 derajat | 33.867 | 406.404 byte | 315.324 |

Angka sel terisi adalah **measured** dari profil node bernama. Jumlah sel dense
dihitung dari bbox Indonesia pada grid yang sama. Grid sparse 0,01 derajat
memakai 128.669 entri, bukan `cols * rows`; dense 0,01 derajat membutuhkan
94,6 MB hanya untuk tabel sel, sehingga dense ditolak.

## 5. Distribusi panjang token

Panjang token puncaknya 6 sampai 8 karakter: 88.711 token (48,0% dari token
unik). Token satu karakter hanya 12. Histogram lengkap ada di JSON.

## 6. Konsekuensi

1. Name FST menyimpan satu baris terbaik per nama; token postings wajib
   menyediakan recall duplikat.
2. Postings `u32` cukup untuk 1,26 juta fitur dan tidak perlu delta varint
   pada v1.
3. Grid sparse 0,01 derajat dipakai untuk reverse 1 km; radius 10 km
   memperbesar jumlah sel dan kandidat, sehingga cap kandidat tetap wajib.
4. Bobot IDF harus dihitung dari 3.453.904 occurrences, bukan dari jumlah
   nama unik.
