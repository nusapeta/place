# Ringkasan

Perubahan apa ini?

## Jenis

- [ ] Perbaikan bug
- [ ] Fitur
- [ ] Dokumentasi
- [ ] Data atau index
- [ ] CI atau build

## Bukti

Untuk perubahan kualitas pencarian atau reverse, isi query/koordinat, hash
index, dan hasil sebelum/sesudah.

```text
query= index_sha256=
before= after=
```

Untuk klaim performa, sertakan mesin, commit, dan perintah reproduksi.

## Checklist

- [ ] `cargo fmt`, `cargo clippy`, `cargo test` lulus.
- [ ] `cargo audit` dan `cargo deny check licenses` lulus.
- [ ] Test regresi ada untuk bug perilaku.
- [ ] Dokumentasi kontrak diperbarui bila perlu.
- [ ] Tidak ada PBF, index, atau file data besar.
- [ ] Commit memakai `Signed-off-by` (DCO).
