# Kontribusi

Repo ini masih di fase fondasi. Kode belum ada. Aturan di bawah berlaku untuk
dokumen sekarang dan untuk kode saat crate dibuat.

## Sebelum mengubah

1. Baca [docs/README.md](docs/README.md) dan dokumen yang terkait perubahan.
2. Cek apakah perubahan menyentuh requirement `FR-xxx` atau `NFR-xxx` di
   [docs/SRS.md](docs/SRS.md).
3. Jika perubahan mengubah repo, bahasa, format index, derivasi `place_id`,
   atau tingkat kompatibilitas API, tulis ADR di
   [docs/decisions/](docs/decisions/).
4. Jangan mengubah `routeid` untuk pekerjaan `placeid`.

## Repo dan GitHub CLI

Repo remote: <https://github.com/arifsans/placeid> (private). Autentikasi dan
operasi GitHub memakai `gh`, bukan token yang ditulis di file repo.

```bash
gh auth login
gh auth status
gh repo view arifsans/placeid
gh repo clone arifsans/placeid
```

`gh auth setup-git` menyiapkan credential helper untuk `git push`. Jangan
menaruh token di `.env`, dokumentasi, atau commit.

Alur perubahan:

```bash
git switch -c docs/gh-workflow
git add CONTRIBUTING.md
git commit -m "docs: document GitHub CLI workflow"
git push -u origin docs/gh-workflow
gh pr create --fill
gh pr checks --watch
gh pr merge --squash --delete-branch
```

Commit baseline dokumen boleh langsung ke `main`. Perubahan berikutnya lewat
branch dan pull request. Jangan memaksa push ke `main`.

Backlog tidak menyimpan status per task. Status ada di issue:

```bash
gh issue list --state open
gh issue create --title "P1-04: spike engine" --body-file /tmp/p1-04.md
```

Gunakan label `status:todo`, `status:doing`, `status:blocked`, dan
`status:done`. Maksimal satu issue berlabel `status:doing` per orang.

## Dokumen

- Bahasa Indonesia. Istilah API, nama field, dan nama file tetap Inggris.
- Tanpa em dash. Gunakan koma, titik dua, atau tanda kurung.
- Angka yang bukan hasil pengukuran diberi label target, perkiraan, atau
  belum diukur.
- Tautan internal memakai path relatif dan harus valid.
- Satu topik punya satu dokumen utama. Jangan menyalin isi dokumen lain.
- Perubahan requirement harus memperbarui matriks traceability di SRS.

## Kode

Saat crate dibuat:

- Format dengan `cargo fmt`.
- Lint dengan `cargo clippy --all-targets -- -D warnings`.
- Test dengan `cargo test`.
- Jangan menambah dependency tanpa alasan yang jelas.
- `unsafe` hanya untuk mmap dan pembacaan record fixed-size. Setiap blok
  `unsafe` harus punya komentar alasan dan test.
- Jangan menulis PBF, index, atau dump data ke repo.
- Jangan menaruh logika HTTP di `lib.rs`.

## Test

- Setiap bug perilaku harus punya test regresi.
- Format index butuh fixture valid dan fixture rusak.
- Normalisasi dan ranking butuh test tabel, bukan hanya satu contoh.
- Perubahan performa harus disertai hasil benchmark di
  [docs/BENCHMARKS.md](docs/BENCHMARKS.md).
- Jangan menguji teks implementasi. Uji perilaku yang terlihat klien.

## Commit

- Pesan ringkas, imperatif, dan menyebut area.
- Contoh: `index: reject section offsets past EOF`.
- Satu commit sebaiknya satu perubahan logis.
- Jangan campur reformat besar dengan perubahan perilaku.

## Pull request

Checklist:

- [ ] Requirement atau ADR terkait sudah diperbarui.
- [ ] `cargo fmt`, `cargo clippy`, dan `cargo test` lulus.
- [ ] Tidak ada PBF, index, atau file data besar.
- [ ] Dokumentasi API dan format diperbarui bila kontrak berubah.
- [ ] Tidak ada klaim performa tanpa pengukuran.
- [ ] Atribusi ODbL tetap ada di respons API.

## Lisensi kontribusi

Dengan mengirim kontribusi, Anda setuju bahwa kontribusi dilisensikan di bawah
MIT untuk kode dan dokumentasi. Data OSM tetap ODbL 1.0 dan tidak disalin ke
repo.
