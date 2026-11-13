# Review NFR v1

Status per 2026-11-13. Angka berasal dari pengukuran di
[P3-EVIDENCE](P3-EVIDENCE.md) dan [P4-EVIDENCE](P4-EVIDENCE.md), kecuali
yang ditandai belum.

| ID | Target | Hasil | Status |
|---|---|---|---|
| NFR-001 | Search p95 ≤ 50 ms pada 20 RPS, 10 menit | 49,047 ms, 8.431 search, 0 error | Lulus dengan catatan |
| NFR-002 | Reverse p95 ≤ 30 ms pada 20 RPS, 10 menit | 28,876 ms, 2.395 reverse, 0 error | Lulus dengan catatan |
| NFR-003 | Index ≤ 600 MB | 141.997.032 B | Lulus |
| NFR-004 | Idle RSS ≤ 50 MB | 6.300 KiB (6,15 MiB) | Lulus |
| NFR-005 | Startup ≤ 1 detik, page cache dingin | 197,93 ms p50, 200,19 ms maksimum setelah `posix_fadvise(DONTNEED)` | Lulus dengan catatan |
| NFR-006 | Build ≤ 15 menit pada 12 worker | 228,42 s, peak RSS 8.189.128 KiB | Lulus |
| NFR-007 | Linux amd64 dan arm64 | amd64 native dan arm64 QEMU: health 200, search, reverse | Lulus dengan catatan |
| NFR-008 | Parser menolak offset di luar batas | 14 test negatif, 400 mutasi fuzz | Lulus |
| NFR-009 | Metrik request, latency, error, ukuran index | `/metrics` diuji | Lulus |
| NFR-010 | Query dan koordinat tidak disimpan default | Test logging dan audit default | Lulus |
| NFR-011 | Atribusi ODbL | Test kontrak field `licence` | Lulus |
| NFR-012 | Proses tunggal tanpa database | Container hanya binary dan index | Lulus |
| NFR-013 | Top-1 ≥ 80%, top-5 ≥ 95% pada 300 query | Search 285: top-1 96,14%, top-5 99,30%; blind 100: 98,00% dan 99,00%; reverse 15: 15/15 | Lulus |

## Catatan

- NFR-001 dan NFR-002 sangat dekat dengan batas. Pada uji 10 menit, search
  p95 49,047 ms hanya 0,953 ms di bawah target, dan reverse 28,876 ms hanya
  1,124 ms di bawah. Beban 11.996 request, 0 error, 19,99 RPS. Variasi mesin
  atau page cache bisa melewati batas; alert p95 dipasang pada ambang target.
  RSS puncak selama uji 150.180 KiB dan CPU 18,01% dari satu core. Gate
  memakai satu run 10 menit, bukan tiga run; pengulangan belum dilakukan.
- NFR-005 memakai `posix_fadvise(POSIX_FADV_DONTNEED)` pada file index sebelum
  setiap run, bukan drop cache global. Halaman index dikeluarkan dari page
  cache, tetapi metadata filesystem, binary, dan dynamic library tetap
  hangat. Angka 197,93 ms karena itu batas atas optimistis untuk cache dingin
  penuh, bukan bukti cold boot menyeluruh. Lima run, tanpa
  `--verify-checksum`, RSS idle 6.120 sampai 6.300 KiB.
- NFR-007 arm64 dijalankan lewat QEMU pada host amd64, bukan hardware arm64.
  Image arm64 menjalankan health, search, reverse, user 65532, dan root
  read-only. Kinerja arm64 tidak diklaim.
- NFR-013 memakai 285 query search dan 15 query reverse. Set blind 100
  dievaluasi satu kali setelah ranker dibekukan: top-1 98%, top-5 99%.
  Reverse dihitung sebagai hit bila nama sesuai atau 404 sesuai label, bukan
  relevance peringkat.
