# ADR 0008: Format index v2 untuk polygon dan geometri jalan

| Field | Value |
|---|---|
| Status | Accepted |
| Tanggal | 2026-11-18 |
| Konteks | Nusapeta Place v2 |

## Konteks

`/reverse` v1 hanya mengukur jarak haversine ke centroid. Untuk jalan panjang,
centroid bukan lokasi terdekat ke titik kueri. `/place/admin/reverse` juga tidak
bisa dibangun hanya dari bbox admin karena bbox tidak membedakan titik di dalam
dan di luar batas.

Perbaikan memerlukan dua section baru dalam index: polygon admin dan polyline
jalan. Reader v1 menolak flag yang tidak dikenal, sehingga index dengan section
baru tidak kompatibel mundur.

## Keputusan

Format index dinaikkan ke `format_version = 2` dengan `MIN_FORMAT_VERSION = 1`.

- Section 10 menyimpan polygon admin (directory 20 byte per entri).
- Section 11 menyimpan polyline jalan (directory 12 byte per entri).
- Header `flags` bit 0 menandakan polygon admin; bit 1 menandakan way lines.
- Reader v2 menerima v1 hanya jika `flags == 0`.
- `/reverse` memakai jarak ke polyline untuk jalan panjang dan centroid untuk
  fitur lain; field `distance_m` ditambahkan.
- `/place/admin/reverse` mengembalikan hierarchy admin dari polygon dan
  menghasilkan 501 `no_admin_polygons` bila section tidak ada.

## Konsekuensi

Positif:

- Jalan panjang menghasilkan fitur terdekat yang benar secara geometris.
- Admin reverse tidak bergantung pada perkiraan bbox.

Negatif:

- Builder harus menulis ulang index untuk memakai fitur baru.
- Reader lama harus diganti untuk membaca index v2.
- Ukuran index bertambah; batas RSS ≤ 2 MB harus diverifikasi pada build penuh.

## Alternatif yang ditolak

| Alternatif | Alasan |
|---|---|
| Menyimpan geometry di file terpisah | Menambah I/O dan sinkronisasi checksum di runtime. |
| Memakai bbox admin untuk reverse | Bbox tidak bisa membedakan titik di luar polygon. |
| Menimpa format v1 tanpa bump | Reader lama akan menolak flag baru tanpa pesan yang jelas. |

## Tinjau ulang

Keputusan ditinjau ulang jika RSS build penuh melebihi 2 MB atau evaluasi
golden menunjukkan regresi pada pasangan jalan pendek.
