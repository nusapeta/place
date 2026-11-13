# Dokumentasi placeid

Dokumen di direktori ini adalah sumber kontrak untuk pekerjaan `placeid`.
Kode belum ada. Saat kode ditulis, requirement dan format di sini menjadi
acuan review.

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

## Status dokumen

| Dokumen | Status | Pemilik topik |
|---|---|---|
| PRD | Draft | masalah, pengguna, scope, metrik produk |
| SRS | Draft | requirement fungsional, NFR, acceptance criteria |
| TRD | Draft | arsitektur, pipeline, ranking, risiko teknis |
| ARCHITECTURE | Draft | diagram dan batas komponen |
| API | Draft | endpoint, parameter, response, error |
| INDEX-FORMAT | Draft | layout index, validasi, versioning |
| BENCHMARKS | Draft | protokol dan hasil pengukuran |
| OPERATIONS | Draft | prosedur runtime dan rebuild |
| ROADMAP | Draft | fase dan exit criteria |
| BACKLOG | Draft | task, ukuran, dependensi, acceptance |
| research/LIGHTWEIGHT-METHODS | Draft riset | perbandingan metode, bukti, rekomendasi |
| ADR | Draft | keputusan yang sudah diambil |

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
alternatif yang ditolak.

ADR yang sudah ada:

- [0001: Repo terpisah dan service Rust](decisions/0001-separate-repo-rust.md)
- [0002: Index mmap khusus](decisions/0002-custom-mmap-index.md)
- [0003: place_id 53-bit](decisions/0003-place-id.md)
- [0004: Subset API Nominatim](decisions/0004-nominatim-subset.md)

## Konvensi penulisan

- Bahasa Indonesia. Istilah API, nama field, dan nama file tetap Inggris.
- Tanpa em dash. Gunakan koma, titik dua, atau tanda kurung.
- Angka yang bukan hasil pengukuran diberi label "target", "perkiraan", atau
  "belum diukur".
- Tautan internal memakai path relatif dan harus valid.
- Data OSM berlisensi ODbL. Jangan menyalin data PBF atau index ke repo.
