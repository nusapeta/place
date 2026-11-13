# Benchmark placeid

| Field | Value |
|---|---|
| Status | Hasil v1 terukur, termasuk beban 10 menit dan arm64 |
| Tanggal | 2026-11-13 |
| Dokumen terkait | [SRS](SRS.md), [TRD](TRD.md) |

Dokumen ini menetapkan cara mengukur `placeid`. Tujuannya mencegah klaim
performa tanpa data dan membuat hasil bisa dibandingkan antar commit.

## 1. Aturan pelaporan

- Setiap hasil harus menyebut commit, versi Rust, versi builder, SHA-256 PBF,
  ukuran index, dan spesifikasi mesin.
- Hasil tanpa perintah yang bisa diulang tidak dipakai sebagai gate.
- Laporkan p50, p95, p99, throughput, error rate, RSS, dan CPU. Jangan hanya
  melaporkan angka terbaik.
- Hasil relevansi harus memakai query set yang disimpan dan versi label yang
  sama.
- Angka target di [SRS](SRS.md) adalah gate, bukan hasil. Tabel hasil di bawah
  diisi setelah pengukuran.

## 2. Mesin referensi

| Peran | Spesifikasi minimum | Catatan |
|---|---|---|
| Build | 12 vCPU, 16 GB RAM, SSD | Sama dengan target NFR-006 |
| Serve | 4 vCPU, 8 GB RAM, SSD | Sama dengan target NFR-001 dan NFR-002 |
| Client load | 4 vCPU, 8 GB RAM, jaringan 1 Gbps | Tidak boleh berbagi CPU dengan server |

Catat model CPU, kernel, dan governor. Jalankan server tanpa container lain
yang berebut CPU. Jika memakai container, beri limit CPU dan memori yang
sesuai.

## 3. Workload

### 3.1 Beban layanan

Campuran request:

| Jenis | Porsi | Contoh |
|---|---:|---|
| Search | 70% | Nama tempat, POI, jalan |
| Reverse | 20% | Titik di jalan dan permukiman |
| Lookup | 10% | `osm_ids` dan `place_ids` |

Prosedur:

1. Build index dari PBF yang dicatat.
2. Start server, tunggu `/healthz` 200.
3. Warm-up 2 menit dengan beban yang sama.
4. Ukur 10 menit pada 20 RPS.
5. Ulangi tiga kali, laporkan median dan rentang. Rilis v1 memakai satu run
   10 menit penuh karena batas waktu; kekurangan pengulangan dicatat di
   [NFR-REVIEW](research/NFR-REVIEW.md) dan tidak diubah menjadi klaim
   stabilitas jangka panjang.

Alat yang dipakai: `scripts/load_test.py` untuk beban tetap campuran,
`scripts/measure_startup.py` untuk startup, `scripts/measure_build.py` untuk
build, dan `/proc/<pid>/status` serta `smaps_rollup` untuk RSS. Alat Python
standar dipakai karena `oha`, `wrk2`, dan `hyperfine` tidak tersedia di mesin
uji dan tidak boleh diklaim sebagai alat yang dipakai.

### 3.2 Build

Ukur dari proses start sampai manifest selesai:

```text
/usr/bin/time -v placeid-build \
  --input indonesia-latest.osm.pbf \
  --output places.idx \
  --manifest places.idx.json \
  --verify
```

Catat wall-clock, peak RSS, ukuran index, jumlah fitur, jumlah fitur dilewati,
dan waktu per pass. Build diulang dua kali untuk memeriksa determinisme.

### 3.3 Startup

Ukur waktu dari exec sampai `/healthz` mengembalikan 200. Lakukan 5 kali.
Jika page cache bisa dikosongkan, catat bahwa cache dingin. Jika tidak, catat
status cache dan jangan menyebut hasil sebagai cold start.

### 3.4 RSS

Setelah 60 detik tanpa traffic:

```text
grep -E 'Rss|Pss|Private' /proc/<pid>/smaps_rollup
```

Catat juga `VmRSS` dari `/proc/<pid>/status`. Bedakan RSS yang berasal dari
mmap index dan heap. Halaman index yang belum disentuh tidak boleh dihitung
sebagai biaya query.

## 4. Query set relevansi

Dataset 300 baris dengan stratifikasi di bawah. Label search adalah nama
kanonik yang diterima dari index, bukan niat manusia independen; batas ini
dicatat di [P4-EVIDENCE](research/P4-EVIDENCE.md).

| Kategori | Jumlah | Contoh |
|---|---:|---|
| Nama tempat dan wilayah | 90 | `Malioboro`, `Sleman`, `Danau Toba` |
| POI dan merek | 90 | `Indomaret`, `Pasar Beringharjo` |
| Jalan | 60 | `Jalan Sudirman`, `Jl. Kaliurang` |
| Alias, singkatan, typo | 45 | `Jl. Malioboro`, `kec sleman` |
| Reverse | 15 | Titik di jalan dan permukiman |

Setiap query punya label fitur yang dimaksud: `osm_type`, `osm_id`, atau daftar
fitur yang dapat diterima. Untuk reverse, label berupa nama jalan atau tempat
yang diharapkan. Minimal 3 dari 15 titik reverse **MUST** berada pada jalan
dengan diagonal bbox di atas 1 km, agar batas centroid terukur, bukan
tersembunyi. Query dan label disimpan di repo sebagai data uji, bukan data
OSM mentah.

Metrik:

- Top-1: hasil pertama adalah fitur yang diberi label.
- Top-5: fitur yang diberi label ada di lima hasil pertama.
- Empty rate: query yang tidak mengembalikan hasil.
- Reverse distance error: jarak antara titik dan fitur yang dikembalikan,
  bila label punya koordinat.

## 5. Hasil

Tabel diisi setelah benchmark pertama. Jangan mengisi dengan perkiraan.

Angka di bawah diukur pada working tree final sebelum commit rilis; commit
implementasi dicatat di tabel. Mesin: AMD Ryzen 5 5600G, 12 thread,
14 GiB RAM, Linux 7.2.15-1-omarchy x64. Input PBF 1.741.650.872 B dengan
SHA-256 `752489d3c4a1e2dd5116eba702b87c8a557dbefba50207e1183e8f6e1f35c774`.
Rust 1.99.1, builder dan server 0.1.0, format index 1.

| Metrik | Target | Hasil | Tanggal | Commit |
|---|---:|---:|---|---|
| Overall p50 / p95 / p99 | belum ditetapkan | 18,974 / 47,553 / 57,558 ms | 2026-11-13 | `55fe1c6` |
| Search p95 | ≤ 50 ms | 49,047 ms, 20 RPS, 600 detik | 2026-11-13 | `55fe1c6` |
| Reverse p95 | ≤ 30 ms | 28,876 ms, 20 RPS, 600 detik | 2026-11-13 | `55fe1c6` |
| Search p99 | belum ditetapkan | 58,224 ms, 20 RPS, 600 detik | 2026-11-13 | `55fe1c6` |
| Reverse p99 | belum ditetapkan | 30,471 ms | 2026-11-13 | `55fe1c6` |
| Lookup p95 / p99 | belum ditetapkan | 28,872 / 30,714 ms | 2026-11-13 | `55fe1c6` |
| Throughput stabil | 20 RPS | 19,99 RPS, 0 error dari 11.996 request | 2026-11-13 | `55fe1c6` |
| Error rate | 0% untuk input valid | 0,00% | 2026-11-13 | `55fe1c6` |
| Index Indonesia | ≤ 600 MB | 141.997.032 B | 2026-11-13 | `55fe1c6` |
| Idle RSS | ≤ 50 MB | 6.300 KiB (6,15 MiB) | 2026-11-13 | `55fe1c6` |
| RSS beban 10 menit | belum ditetapkan | 150.180 KiB puncak | 2026-11-13 | `55fe1c6` |
| CPU beban 10 menit | belum ditetapkan | 18,01% dari satu core | 2026-11-13 | `55fe1c6` |
| Startup | ≤ 1 detik | 197,93 ms p50, 200,19 ms maksimum, fadvise DONTNEED | 2026-11-13 | `55fe1c6` |
| Build Indonesia | ≤ 15 menit | 228,42 s, peak RSS 8.189.128 KiB | 2026-11-13 | `55fe1c6` |
| Top-1 relevance | ≥ 80% | 96,14% (274/285) | 2026-11-13 | `55fe1c6` |
| Top-5 relevance | ≥ 95% | 99,30% (283/285) | 2026-11-13 | `55fe1c6` |
| Blind top-1 | ≥ 80% | 98,00% (98/100) | 2026-11-13 | `55fe1c6` |
| Blind top-5 | ≥ 95% | 99,00% (99/100) | 2026-11-13 | `55fe1c6` |
| Determinisme index | 100% | 2 build byte-identik, file `a732b4ec...` | 2026-11-13 | `55fe1c6` |
| Image amd64 | berjalan | digest `49cb923f...`, 147 MB | 2026-11-13 | `55fe1c6` |
| Image arm64 | berjalan | digest `134899f4...`, 36,7 MB, QEMU | 2026-11-13 | `55fe1c6` |

Rincian dataset, per kategori, reverse, dan beban ada di
[P4-EVIDENCE](research/P4-EVIDENCE.md). Status NFR ada di
[NFR-REVIEW](research/NFR-REVIEW.md).

## 6. Analisis regresi

- Bandingkan hasil pada commit yang sama dengan mesin yang sama.
- Jika latency naik lebih dari 10%, cek jumlah kandidat, ukuran postings, dan
  perubahan bobot.
- Jika index membesar lebih dari 5%, cek duplikasi string, varian nama, dan
  section admin.
- Jika relevansi turun, pecah per kategori. Jangan menyimpulkan dari agregat.
- Simpan raw output benchmark sebagai artefak CI, bukan di git.

## 7. Batas

Benchmark ini mengukur layanan pada satu mesin, bukan jaminan SLA. Hasil pada
hardware lain bisa berbeda. Untuk klaim publik, sertakan spesifikasi mesin dan
perintah yang dipakai.
