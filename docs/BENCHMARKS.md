# Benchmark placeid

| Field | Value |
|---|---|
| Status | Draft, belum ada hasil |
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
5. Ulangi tiga kali, laporkan median dan rentang.

Alat yang dipakai: `oha` atau `wrk2` untuk beban tetap, `hyperfine` untuk
startup, dan `smaps_rollup` untuk RSS. Catat versi alat.

### 3.2 Build

Ukur dari proses start sampai manifest selesai:

```text
/usr/bin/time -v placeid-build \
  --pbf indonesia-latest.osm.pbf \
  --out places.idx \
  --manifest places.idx.json \
  --workers 12
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

Buat 300 query berlabel manual dengan stratifikasi:

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

| Metrik | Target | Hasil | Tanggal | Commit |
|---|---:|---:|---|---|
| Search p95 | ≤ 50 ms | belum diukur | | |
| Reverse p95 | ≤ 30 ms | belum diukur | | |
| Search p99 | belum ditetapkan | belum diukur | | |
| Throughput stabil | 20 RPS | belum diukur | | |
| Error rate | 0% untuk input valid | belum diukur | | |
| Index Indonesia | ≤ 600 MB | belum diukur | | |
| Idle RSS | ≤ 50 MB | belum diukur | | |
| Startup | ≤ 1 detik | belum diukur | | |
| Build Indonesia | ≤ 15 menit | belum diukur | | |
| Top-1 relevance | ≥ 80% | belum diukur | | |
| Top-5 relevance | ≥ 95% | belum diukur | | |
| Determinisme index | 100% | belum diukur | | |

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
