# Keamanan

## Melaporkan masalah

Jangan membuka issue publik untuk kerentanan yang bisa dieksploitasi. Gunakan
GitHub private security advisory jika repo tersedia di GitHub. Jika tidak,
hubungi maintainer secara langsung dan sertakan:

- Versi commit.
- Langkah reproduksi.
- Dampak yang mungkin.
- Apakah masalah bisa dipicu lewat HTTP atau hanya lewat index yang rusak.

Akan ada konfirmasi penerimaan, lalu perbaikan dan catatan rilis setelah
masalah tertangani. Tidak ada janji waktu respons.

## Model ancaman

| Aset | Ancaman | Mitigasi |
|---|---|---|
| Proses server | Input HTTP berbahaya | Validasi panjang, rentang, dan tipe |
| Index | File rusak atau dimodifikasi | Magic, versi, batas section, checksum |
| Parser | Offset di luar batas | Validasi ketat, `unsafe` terbatas, fuzz |
| Dependency | Crate berbahaya atau rentan | `cargo audit`, review dependency |
| Container | Akses tulis atau root | Read-only, non-root, tanpa shell |
| Data | Penyalahgunaan data OSM | Atribusi ODbL, tidak menyimpan query |

## Batas

`nusapeta-place` tidak menangani autentikasi, TLS, atau rate limit. Itu tanggung
jawab gateway. Jangan mengekspos port `nusapeta-place` langsung ke internet.

## Yang dianggap kerentanan

- Remote crash atau panic dari request HTTP valid.
- Out-of-bounds read dari index yang dibuka dengan flag verifikasi.
- Eksekusi kode dari input PBF atau index.
- Kebocoran path filesystem atau stack trace di respons.
- Denial of service yang tidak bisa dibatasi dengan limit request.

## Yang bukan kerentanan

- Hasil pencarian yang salah karena data OSM tidak lengkap.
- Latency di luar target pada hardware yang tidak memenuhi spesifikasi.
- Klien yang mengirim `format` tidak didukung dan mendapat 400.
- `place_id` berbeda dari Nominatim. Itu perbedaan desain yang terdokumentasi.

## Dependency

- Pin versi di `Cargo.lock`.
- Jalankan `cargo audit` sebelum rilis.
- Hindari crate yang tidak dirawat atau punya `unsafe` besar tanpa alasan.
- Catat dependency baru di pull request.

## Data dan privasi

- Layanan tidak menyimpan query, koordinat, atau hasil.
- Log default tidak memuat teks query atau koordinat.
- Metrik tidak memakai query sebagai label.
- PBF dan index tidak boleh masuk git.
