# Glosarium

| Istilah | Arti |
|---|---|
| Admin hierarchy | Rantai wilayah administratif dari fitur ke parent, misalnya kelurahan, kecamatan, kabupaten, provinsi |
| Admin level | Nilai `admin_level` OSM. v1 memakai level 4 sampai 7 |
| Alias | Nama alternatif untuk fitur, termasuk `alt_name`, `old_name`, `short_name`, dan `brand` |
| Bounding box | Kotak minimum yang memuat fitur, disimpan sebagai empat koordinat |
| Candidate | Row place yang lolos lookup FST dan postings sebelum ranking |
| Deterministik | Input dan versi builder yang sama menghasilkan index body yang sama |
| Display name | Nama fitur beserta rantai admin yang disusun saat query |
| FST | Finite State Transducer, struktur untuk memetakan string ke nilai dan mendukung pencarian prefix |
| Forward search | Pencarian berdasarkan teks nama |
| Geocoding | Mengubah nama atau alamat menjadi koordinat |
| Grid | Pembagian ruang menjadi sel untuk mempercepat reverse geocoding |
| House number | Nomor bangunan dari `addr:housenumber`. Tidak dicari di v1 |
| Importance | Skor relatif fitur yang dipakai sebagai sinyal ranking |
| Index | File `places.idx` yang dipetakan read-only oleh server |
| Interpolasi alamat | Menghitung nomor rumah di antara dua titik. Tidak ada di v1 |
| JSONv2 | Format respons Nominatim yang menjadi acuan subset API |
| Lookup | Pengambilan fitur berdasarkan `place_id` atau `osm_ids` |
| Manifest | File JSON berisi provenance, versi, jumlah, dan checksum index |
| mmap | Memory-mapped file, halaman dibaca saat disentuh, tidak disalin seluruhnya ke heap |
| Nominatim | Geocoder OSM yang menjadi acuan kompatibilitas terbatas |
| ODbL | Open Database License, lisensi data OpenStreetMap |
| OSM | OpenStreetMap |
| OSM ID | Identifier objek OSM, unik per tipe |
| PBF | Format biner data OSM, biasanya `.osm.pbf` |
| Place | Fitur bernama yang bisa dikembalikan API |
| `place_id` | Identifier 53-bit turunan `(osm_type, osm_id)` |
| Postings | Daftar row index yang terkait dengan token atau sel grid |
| Promote | Mengganti index aktif dengan rilis baru |
| p95 | Persentil ke-95 dari distribusi latency |
| Rebuild | Membangun ulang index dari PBF |
| Reverse geocoding | Mengubah koordinat menjadi fitur atau nama terdekat |
| Rollback | Mengembalikan index aktif ke rilis sebelumnya |
| Row index | Posisi record di section places, dipakai postings dan FST |
| RSS | Resident Set Size, memori fisik yang dipakai proses |
| Snapshot | Satu pasangan index dan manifest pada waktu tertentu |
| Token | Potongan nama hasil normalisasi yang dipakai untuk pencarian |
| Top-K | K hasil teratas setelah ranking |
| Trailer | 32 byte SHA-256 di akhir file index |
| Zero-copy | Membaca record langsung dari mmap tanpa menyalin ke struktur heap |
