# Dokumentasi Nusapeta Place

Dokumen di direktori ini adalah sumber kontrak dan bukti untuk `nusapeta-place`.
Implementasi v1 sudah ada di `src/`; requirement dan format di sini menjadi
acuan review dan operasi.

## Urutan baca

1. [PRD](PRD.md): alasan produk, pengguna, scope, dan metrik sukses.
2. [SRS](SRS.md): requirement bernomor yang bisa diuji.
3. [TRD](TRD.md): rancangan teknis dan trade-off.
4. [ARCHITECTURE](ARCHITECTURE.md): diagram komponen dan alur data.
5. [API](API.md): kontrak HTTP.
6. [INDEX-FORMAT](INDEX-FORMAT.md): kontrak file index.
7. [BENCHMARKS](BENCHMARKS.md): cara mengukur dan melaporkan performa.
8. [OPERATIONS](OPERATIONS.md): build, deploy, rebuild, rollback.
9. [ROADMAP](ROADMAP.md): urutan pekerjaan dan exit criteria.
10. [BACKLOG](BACKLOG.md): task yang bisa dilacak, ukuran, dan dependensi.
11. [research/LIGHTWEIGHT-METHODS](research/LIGHTWEIGHT-METHODS.md): riset
    metode ringan dan akurat.
12. [GLOSSARY](GLOSSARY.md): istilah.
13. [DEPENDENCIES](DEPENDENCIES.md): lisensi, audit, dan atribusi.
14. [research/P3-EVIDENCE](research/P3-EVIDENCE.md): bukti kualitas index.
15. [research/P4-EVIDENCE](research/P4-EVIDENCE.md): relevance, reverse, beban,
    dan release drill.
16. [research/NFR-REVIEW](research/NFR-REVIEW.md): status gate NFR.
17. [releases/v0.2.0](releases/v0.2.0.md): catatan rilis 0.2.0 (index format
    v2, admin polygon, way line, suggest).
18. [releases/v0.1.0](releases/v0.1.0.md): catatan rilis v1.

## Status dokumen

| Dokumen | Status | Pemilik topik |
|---|---|---|
| PRD | v1 | masalah, pengguna, scope, metrik produk |
| SRS | v1 | requirement fungsional, NFR, acceptance criteria |
| TRD | v1 | arsitektur, pipeline, ranking, risiko teknis |
| ARCHITECTURE | v1 | diagram dan batas komponen |
| API | v1 | endpoint, parameter, response, error |
| INDEX-FORMAT | v1 | layout index, validasi, versioning |
| BENCHMARKS | Hasil terukur | protokol dan hasil pengukuran |
| OPERATIONS | v1 | prosedur runtime dan rebuild |
| ROADMAP | v1 | fase dan exit criteria |
| BACKLOG | Aktif | task, ukuran, dependensi, acceptance |
| research/LIGHTWEIGHT-METHODS | Riset selesai | perbandingan metode, bukti, rekomendasi |
| ADR | Accepted | keputusan yang sudah diambil |

`Draft` berarti bisa berubah sebelum kode v1. Perubahan yang mengubah kontrak
eksternal atau format file wajib disertai catatan di dokumen terkait dan, bila
keputusannya arsitektural, ADR baru.

## Aturan sumber kebenaran

Setiap topik hanya punya satu dokumen utama. Dokumen lain merujuk, bukan
menyalin.

| Topik | Sumber utama |
|---|---|
| Masalah, pengguna, non-goals | PRD |
| ID requirement dan acceptance criteria | SRS |
| Keputusan teknis dan trade-off | TRD dan ADR |
| Bentuk diagram | ARCHITECTURE |
| Kontrak HTTP | API |
| Kontrak file index | INDEX-FORMAT |
| Angka hasil pengukuran | BENCHMARKS |
| Prosedur operasi | OPERATIONS |
| Urutan fase dan exit criteria | ROADMAP |
| Task yang bisa dilacak dan acceptance | BACKLOG |
| Bukti dan rekomendasi metode | research/LIGHTWEIGHT-METHODS |

## Aturan requirement

- Requirement fungsional memakai ID `FR-xxx`; nonfungsional memakai `NFR-xxx`.
- Kata `MUST`, `MUST NOT`, `SHOULD`, `SHOULD NOT`, dan `MAY` mengikuti
  RFC 2119.
- Setiap requirement punya acceptance criterion yang bisa dijalankan atau
  diperiksa.
- Perubahan requirement harus memperbarui matriks traceability di SRS.
- Target performa adalah gate awal sampai diukur. Hasil pengukuran masuk ke
  BENCHMARKS, bukan diklaim di PRD.

## Aturan keputusan

Keputusan yang mengubah bentuk repo, bahasa, format index, derivasi
`place_id`, atau tingkat kompatibilitas API harus punya ADR di
[decisions/](decisions/). Format ADR: konteks, keputusan, konsekuensi,
alternatif yang ditolak. Mulai ADR baru dari
[decisions/TEMPLATE.md](decisions/TEMPLATE.md).

Aturan review ADR:

- Satu ADR untuk satu keputusan. Jangan menggabungkan keputusan format dengan
  keputusan operasi.
- Status `Accepted` hanya setelah ada data atau test yang mendukung.
- Keputusan yang dibatalkan diberi status `Superseded` dan menunjuk ADR
  pengganti.
- Angka di bagian konteks harus berlabel target, perkiraan, atau hasil
  pengukuran.
- Perubahan format index wajib memperbarui [INDEX-FORMAT](INDEX-FORMAT.md) pada
  commit yang sama.

ADR yang sudah ada:

- [0001: Repo terpisah dan service Rust](decisions/0001-separate-repo-rust.md)
- [0002: Index mmap khusus](decisions/0002-custom-mmap-index.md)
- [0003: place_id 53-bit](decisions/0003-place-id.md)
- [0004: Subset API Nominatim](decisions/0004-nominatim-subset.md)
- [0005: Engine pencarian](decisions/0005-engine.md)
- [0006: Fuzzy v1](decisions/0006-fuzzy.md)
- [0007: Importance](decisions/0007-importance.md)
- [0008: Format index v2](decisions/0008-index-format-v2.md)

## Konvensi penulisan

- Bahasa Indonesia. Istilah API, nama field, dan nama file tetap Inggris.
- Tanpa em dash. Gunakan koma, titik dua, atau tanda kurung.
- Angka yang bukan hasil pengukuran diberi label "target", "perkiraan", atau
  "belum diukur".
- Tautan internal memakai path relatif dan harus valid.
- Data OSM berlisensi ODbL. Jangan menyalin data PBF atau index ke repo.
