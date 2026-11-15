# Kontribusi

Nusapeta Place adalah geocoder OSM untuk Indonesia: satu crate Rust, satu file
index `mmap`, tanpa database. Kontribusi kecil tetap berarti, terutama laporan
nama tempat yang hilang dan hasil reverse yang salah.

## Sebelum mengubah

1. Baca [docs/README.md](docs/README.md) dan dokumen yang terkait perubahan.
2. Cek requirement `FR-xxx`/`NFR-xxx` di [docs/SRS.md](docs/SRS.md) dan
   [docs/BACKLOG.md](docs/BACKLOG.md).
3. Perubahan format index, derivasi `place_id`, atau kontrak HTTP butuh diskusi
   issue dan ADR di [docs/decisions/](docs/decisions/).
4. Jangan mengubah hasil pengukuran di `docs/BENCHMARKS.md` atau
   `docs/releases/`; itu snapshot. Perbaikan penjelasan ada di dokumen lain.
5. Jangan menyentuh repo Nusapeta Route untuk pekerjaan Place.

## Setup

Toolchain dipin di `rust-toolchain.toml`: Rust 1.99.1 dengan `rustfmt` dan
`clippy`.

```bash
git clone https://github.com/nusapeta/place.git
cd place
rustup show
cargo build --release --bins
cargo test --all-features
```

Perintah yang dipakai CI:

```bash
cargo fmt --all -- --check
cargo clippy --all-targets --all-features -- -D warnings
cargo test --all-features
cargo audit
cargo deny check licenses
python3 scripts/check_consistency.py
```

## Aturan angka

Setiap PR yang mengubah kualitas pencarian atau reverse wajib menyertakan:

- query atau koordinat yang diuji;
- PBF/index yang dipakai (hash dari manifest);
- keluaran sebelum dan sesudah, bukan hanya "lebih baik";
- untuk klaim performa: mesin, commit, dan perintah reproduksi.

Angka tanpa mesin, commit, dan perintah ditolak. Klaim NFR mengikuti
[docs/BENCHMARKS.md](docs/BENCHMARKS.md); jangan mengubah angka beku.

## Kode

- `cargo fmt` wajib; jangan reformat berkas yang tidak disentuh.
- Setiap bug perilaku butuh test regresi.
- `unsafe` hanya untuk mmap dan pembacaan record fixed-size. Setiap blok
  `unsafe` wajib punya komentar alasan dan test.
- Jangan menambah dependency tanpa alasan. Dependency baru harus lulus
  `cargo deny check licenses` dan dicatat di [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md).
- Jangan menaruh domain pribadi, IP LAN, token, PBF, atau index di repo.
  Gunakan `place.example.com` dan `127.0.0.1`.

## Pull request

- [ ] `cargo fmt`, `cargo clippy`, dan `cargo test` lulus.
- [ ] Perubahan kualitas menyertakan query + index + hasil sebelum/sesudah.
- [ ] Dokumentasi API atau format diperbarui bila kontrak berubah.
- [ ] Tidak ada PBF, index, atau file data besar.
- [ ] Commit memakai `Signed-off-by` (DCO).

Commit memakai pesan imperatif dan menyebut area, misalnya
`search: cap prefix expansion at 64 terms`.

## DCO

Repositori memakai Developer Certificate of Origin 1.1, tanpa CLA. Setiap
commit menyertakan baris:

```text
Signed-off-by: Nama Anda <email@contoh.com>
```

Tambahkan dengan `git commit -s`. Kontribusi tanpa sign-off tidak digabung.

## Lisensi

Dengan mengirim kontribusi, Anda setuju kontribusi dilisensikan MIT. Data OSM
dan index turunannya tetap ODbL 1.0; lihat [docs/DEPENDENCIES.md](docs/DEPENDENCIES.md).
