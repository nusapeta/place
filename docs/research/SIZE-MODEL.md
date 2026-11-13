# Model ukuran index v1

| Field | Value |
|---|---|
| Status | Model |
| Tanggal | 2026-11-13 |
| Target | NFR-003, maksimum 600 MB |
| Sumber angka | [CORPUS-PROFILE](CORPUS-PROFILE.md) |

Model ini memakai angka korpus terukur dan layout `INDEX-FORMAT.md`. Semua
hasil adalah **estimate** dari rumus, bukan hasil build penuh. Hasil build
penuh dicatat di [BENCHMARKS](../BENCHMARKS.md).

## 1. Rumus per section

| Section | Rumus | Angka |
|---|---|---:|
| places | `1.263.949 * 64 B` | 80,9 MB |
| admin | `9.405 * 40 B` | 0,38 MB |
| strings | 645.238 nama unik + dictionary, dedup | 21,6 MB |
| variants | `(1.263.949 + 9.405 + 118.348) * 12 B` | 16,7 MB |
| token FST | 184.938 key terukur | 1,19 MB |
| name FST | 612.877 key terukur | 6,69 MB |
| token postings | `3.453.904 * 4 B` | 13,8 MB |
| grid postings | `1.263.949 * 4 B` | 5,1 MB |
| grid sparse | `(128.669 * 12 B) + 40 B` | 1,54 MB |
| header dan trailer | tetap | 0,0003 MB |
| **Total** | | **148,0 MB** |

Total 148,0 MB memakai 24,7% dari batas 600 MB. Margin 452,0 MB (75,3%).
Jika semua fitur bernama, bukan hanya node, menempati sel grid, grid sparse
masih di bawah 7 MB untuk 550.000 sel. Skenario itu tetap di bawah 154 MB.

## 2. Skenario terburuk

| Komponen | Asumsi buruk | Ukuran |
|---|---|---:|
| places | 1.263.949 record | 80,9 MB |
| variants | semua fitur punya 2 varian | 33,4 MB |
| strings | 30 MB | 30,0 MB |
| postings | 2 kali occurrences | 27,6 MB |
| FST | 2 kali ukuran terukur | 15,8 MB |
| grid | 600.000 sel | 7,2 MB |
| admin | 10.000 record | 0,4 MB |
| **Total** | | **195,3 MB** |

Skenario terburuk masih 32,6% dari batas. Tidak ada komponen yang mendekati
600 MB, sehingga format v1 tidak perlu kompresi tambahan sebelum build penuh.

## 3. Keputusan model

1. Places dan variants adalah komponen terbesar. Menambah field 8 byte ke
   place menambah 10,1 MB, jadi field baru wajib punya alasan terukur.
2. Name FST 6,69 MB lebih mahal daripada token FST 1,19 MB. Duplikasi nama
   ditangani postings, bukan dengan memperbesar value FST.
3. Grid dense 0,01 derajat untuk Indonesia memakai puluhan juta sel. Grid
   sparse wajib.
4. Model ini tidak menghitung page cache atau RSS. NFR-004 diukur terpisah.
