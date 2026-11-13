# Bukti P4: evaluasi dan release

Mesin: AMD Ryzen 5 5600G, 12 thread, 14 GiB RAM, Linux 7.2.15-1-omarchy
x64. Index yang diuji adalah `/tmp/placeid-full3/places.idx` berukuran
141.997.032 B dengan SHA-256 file
`a732b4ecaa7c1e9e36f7b475ab891c65d1f485744cc3cbdb9da69eac4d8b836d` dan
body hash manifest
`0df44207907a007459f53285ad727a31441138456bd5f017d408082d00c62bca`.

## Dataset

| Bagian | Jumlah | Sumber |
|---|---:|---|
| Search tune | 150 | 100 query awal, 30 admin, 10 jalan, 40 POI |
| Search holdout awal | 135 | 30 admin, 50 POI, 50 jalan, 45 alias |
| Reverse holdout | 15 | 14 titik darat, 1 titik tanpa fitur dalam radius |
| Total terkontrak | 300 | 285 search, 15 reverse |
| Konfirmasi holdout | 150 | 40 admin, 80 POI, 30 jalan |
| Blind satu kali | 100 | 25 admin, 40 POI, 35 jalan |

Setiap baris search punya kolom kelima berisi daftar `place_id` yang
diterima. Kolom itu dihasilkan oleh `scripts/add_label_ids.py` terhadap index
penuh; satu label blind tidak menemukan nama kanonik yang persis, yaitu
`jalan pucung 1 munggang` yang tampil sebagai `Jln Pucung 1 Munggang`.

Holdout awal sudah dipakai untuk kualifikasi label dan penemuan kekurangan,
sehingga bukan holdout buta. Set `konfirmasi holdout` juga sudah dilihat saat
label admin kanonik diperbaiki. Karena itu bukti akhir yang tidak dipakai
menyesuaikan ranker adalah `blind satu kali`: dibuat setelah ranker dibekukan,
dievaluasi satu kali, dan tidak ada perubahan peringkat setelahnya.

Label adalah nama kanonik atau nama yang diterima dari fitur yang ada di
index, bukan label niat manusia independen. Query hasil sampling nama memakai
bentuk normalisasi index; label menerima bentuk kanonik dengan tanda baca dan
singkatan. Dataset ini cukup untuk regresi peringkat, tetapi tidak mengukur
niat pengguna bebas. Query alias (`jkt`, `soetta`, `tugu jogja`) adalah
minoritas dan beberapa sengaja sulit; hasilnya dilaporkan apa adanya.

## P4-03 Relevance

Diukur tanpa beban konkuren, setelah server hangat.

| Split | Query | Top-1 | Top-5 | p50 | p95 | p99 |
|---|---:|---:|---:|---:|---:|---:|
| Tune | 150 | 145 (96,67%) | 149 (99,33%) | 17,256 ms | 30,598 ms | 33,884 ms |
| Holdout awal | 135 | 129 (95,56%) | 134 (99,26%) | 17,340 ms | 20,904 ms | 28,919 ms |
| Seluruh search | 285 | 274 (96,14%) | 283 (99,30%) | tidak dipakai | tidak dipakai | tidak dipakai |
| Konfirmasi | 150 | 145 (96,67%) | 147 (98,00%) | tidak diukur | 29,233 ms | tidak diukur |
| Blind | 100 | 98 (98,00%) | 99 (99,00%) | tidak diukur | 28,800 ms | tidak diukur |

Latency seluruh set 285 dan set konfirmasi tidak dipakai untuk gate karena
keduanya dijalankan sebelum isolasi akhir; top-1 dan top-5 tetap sah karena
tidak terpengaruh beban. Semua set lain diukur tanpa beban konkuren.

Per kategori pada split tune: region 87/90 top-1 dan 89/90 top-5, POI 48/50
dan 50/50, jalan 10/10 dan 10/10. Pada holdout awal: POI 40/40 dan 40/40,
jalan 50/50 dan 50/50, alias 39/45 dan 44/45. Pada blind: region 25/25,
jalan 34/35, POI 39/40 top-1; top-5 jalan 34/35 dan POI 40/40.

Perubahan ranker yang diuji:

- Cakupan token penuh mendapat bonus satu kali bobot cakupan. Kandidat yang
  cocok sebagian tidak mendapat bonus.
- `effective_tokens` memetakan `jogja`, `jogjakarta`, `jkt`, `sby`, `jl`, dan
  `jln` ke bentuk yang dipakai index.
- Awalan kualifikasi admin (`kota`, `kabupaten`, `kecamatan`, `kelurahan`,
  `provinsi`, `kec`, `kel`, `kab`) tidak lagi menjadi bagian wajib query bila
  kandidat kanonik tidak memuatnya.
- Query yang diawali `kabupaten` memberi bonus 0,75 pada kandidat
  `boundary/administrative`, sehingga batas kabupaten mendahului kota dengan
  nama sama. Bonus ini diuji pada split tune dan diterapkan sebelum set blind
  dibuat.

Kegagalan top-1 yang tersisa: `kota tua` mengembalikan halte `Kota` sebelum
`Kota Tua Jakarta`; `tugu jogja` mengembalikan stasiun `Yogyakarta` sebelum
`Tugu Pal Putih`; `taman nasional komodo` mengembalikan kantor balai sebelum
fitur taman; `sarinah` mengembalikan halte `M.H. Thamrin`; `soetta`
mengembalikan `Buana Soetta Barat`. Semua kecuali `soetta` masih masuk top-5
atau diterima setelah sinonim. Dua top-5 miss berasal dari label admin
`kabupaten pakpak bharat` dan `kabupaten labuhanbatu utara` yang tidak
mengembalikan nama kabupaten yang sama.

## P4-02 Reverse

15 titik, radius default 1000 m. Satu titik Danau Toba tidak punya fitur
bernama dalam radius dan mengembalikan 404; itu dihitung sebagai hasil benar.

| Metrik | Nilai |
|---|---:|
| Hit nama | 15/15 |
| 404 sesuai label | 1 |
| Jarak centroid p50 | 26,1 m |
| Jarak centroid p95 | 167,3 m |
| Jarak centroid maksimum | 167,3 m |
| p95 jarak kategori place | 79,6 m |
| p95 jarak kategori road | 167,3 m |
| p50 latency | 0,627 ms |
| p95 latency | 7,55 ms |

Jarak besar berasal dari jalan panjang dan titik yang centroid-nya jauh,
terutama `Jalan Pabean` 167 m. Ini keterbatasan reverse centroid yang sudah
dicatat: jarak ke centroid bukan jarak ke geometri terdekat.

## P4-04 Duplikat

`GET /search?q=indomaret&limit=5` pada index penuh mengembalikan 5 hasil dan
5 `place_id` unik. Tidak ada ID ganda dalam satu respons.

## P4-05 Beban campuran

Beban 70% search, 20% reverse, 10% lookup pada 20 RPS selama 600 detik setelah
warmup 30 detik, 4 koneksi keep-alive, server final commit `55fe1c6`:

| Metrik | Nilai |
|---|---:|
| Request | 11.996 |
| Throughput | 19,99 RPS |
| Error | 0 (0,00%) |
| Overall p50 / p95 / p99 | 18,974 / 47,553 / 57,558 ms |
| Search p50 / p95 / p99 | 21,511 / 49,047 / 58,224 ms |
| Reverse p50 / p95 / p99 | 2,470 / 28,876 / 30,471 ms |
| Lookup p50 / p95 / p99 | 2,508 / 28,872 / 30,714 ms |
| Peak RSS saat beban | 150.180 KiB |
| CPU proses | 18,01% dari satu core |

Angka CPU dihitung dari `utime+stime` dibagi wall time, jadi 18,87% berarti
sekitar seperlima satu core pada 20 RPS. Peak RSS 146,7 MiB berasal dari page
cache index yang termap saat beban, bukan anon memory. NFR-004 hanya mengikat
RSS idle, yang terukur 6,1 MiB. Uji 60 detik sebelumnya memberi search p95
48,798 ms dan reverse 29,326 ms; arahnya konsisten.

## P4-07 Release candidate

- Manifest, checksum file, dan catatan rilis tersedia. File rilis:
  `places.idx` 141.997.032 B, `places.idx.json`, dan `places.idx.sha256`.
- Checksum file memakai hash file penuh `a732b4ec...`; manifest memakai body
  hash `0df442079...`. Promote memverifikasi checksum file sebelum symlink
  berubah.
- Image amd64 `placeid:0.1`, digest manifest
  `sha256:49cb923f34ba7d6b015df77d82b86e1d845c4e53b0adb4ecb5382f9fb7e91ea5`,
  ukuran 147 MB, dibangun dari commit `55fe1c6`.
- Audit dependency dan lisensi ada di [DEPENDENCIES](../DEPENDENCIES.md);
  `cargo audit` menemukan 0 vulnerability dan 1 warning yang tidak terpakai.
- Image arm64 `placeid:arm64`, digest manifest
  `sha256:134899f4df5c5f18baae8aff05aa8a20376c97201e5097fde98074def9b44584`,
  ukuran 36,7 MB. Smoke pada QEMU: health 200, search `monas` mengembalikan
  Monas, reverse Jakarta, user 65532, root read-only. Kinerja tidak diklaim.

## P4-08 Promote dan rollback

Uji pada `/tmp/release-test`:

1. Rilis A dipromosikan, `current` menunjuk `build-a`.
2. Rilis B dengan checksum valid dipromosikan, `current` menunjuk `build-b`,
   `previous` menyimpan `build-a`.
3. `rollback_release.sh` mengembalikan `current` ke `build-a`.
4. Promote rilis B saat server masih memetakan A tidak mengubah
   `index_sha256`; server tetap memakai mmap lama.
5. Setelah restart, `index_sha256` berubah menjadi checksum manifest B.
6. Rollback ke A lalu restart mengembalikan `index_sha256` A.
7. Rilis dengan checksum salah ditolak sebelum symlink berubah.
8. Index dengan byte terakhir diubah gagal start dengan
   `index checksum mismatch` saat `--verify-checksum`.
9. Index valid dengan `--verify-checksum` mencapai status listening dalam
   kurang dari 3 detik.

Promote dan rollback memakai symlink sementara lalu `mv -Tf`, sehingga
pergantian nama atomik. Service tetap perlu restart agar mmap lama dilepas;
tidak ada hot reload pada v1.

## P5-05 Rebuild kedua dan drill dua replika

Build keempat dari kode final selesai dalam 228,42 s dengan puncak RSS
8.189.128 KiB. File index identik byte demi byte dengan build ketiga:
SHA-256 file `a732b4ec...` dan body manifest `0df442079...`. Ini memenuhi
syarat rebuild deterministik.

Drill dua replika memakai satu host, dua proses, port 18130 dan 18131, serta
dua direktori rilis. Ini simulasi rolling restart, bukan load balancer
produksi:

1. Kedua replika melayani rilis A dengan `built_at 2026-11-13T15:25:22Z`.
2. Rilis B dipromosikan atomik. Tanpa restart, kedua replika tetap melayani
   A.
3. Replika 1 dimatikan dan dijalankan ulang; selama proses itu replika 2
   mengembalikan `/healthz` 200. Setelah hidup, replika 1 melayani B.
4. Langkah yang sama dilakukan pada replika 2 dengan replika 1 sebagai
   penjaga.
5. Kedua replika berakhir pada rilis B dengan `built_at
   2026-11-13T15:48:35Z`, dan tidak ada sampel health gagal.

Karena index kedua build identik, pembeda yang dipakai adalah `built_at` dari
manifest, bukan `index_sha256`. Pada produksi, load balancer **MUST** berhenti
mengirim traffic ke replika yang sedang restart; drill ini membuktikan replika
lain tetap sehat, bukan membuktikan load balancer sudah dikonfigurasi.

## P5 Operasi berulang

- P5-01: perintah rebuild di [OPERATIONS](../OPERATIONS.md) §6.1 dieksekusi
  pada build keempat. Output manifest diverifikasi dengan `--verify`, jumlah
  fitur 1.226.755, checksum body `0df442079...`.
- P5-02: `scripts/promote_release.sh` menolak checksum salah, memakai symlink
  sementara dan `mv -Tf`. `scripts/rollback_release.sh` mengembalikan
  `current` ke `previous`. Keduanya dilatih pada `/tmp/release-test3` dan
  drill dua replika.
- P5-03: `ops/prometheus-alerts.yml` memuat alert error rate, search p95,
  reverse p95, index lebih tua dari 45 hari, proses down, dan working set
  480 MiB. File lolos parse YAML.
- P5-04: `ops/grafana-placeid.json` memuat request, latency, error, ukuran
  index, fitur, kandidat `placeid_search_candidates`, umur index, dan retensi.
  Log request tidak memuat query atau koordinat, hanya request ID, method,
  endpoint, status, dan durasi.
- P5-05: rebuild deterministik dan drill dua replika di bagian sebelumnya.
