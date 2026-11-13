# ADR 0006: fuzzy search v1

| Field | Value |
|---|---|
| Status | Accepted |
| Tanggal | 2026-11-13 |
| Konteks | placeid v1 |

## Konteks

P1-05 meminta perbandingan prefix-only, Levenshtein automaton `fst`, dan
SymSpell deletion index pada 45 query typo/alias. Pengukuran memakai token FST
index Indonesia penuh: 205.491 token. Query terdiri dari 35 typo satu edit dan
10 alias populer. Recall dihitung pada 5 kandidat teratas setelah diurutkan
berdasarkan frekuensi token.

**Measured** pada 13 November 2026:

| Strategi | Recall@5 | p95 |
|---|---:|---:|
| Prefix-only | 4/45 | 0,008 ms |
| Levenshtein 1 | 28/45 | 0,343 ms |
| Levenshtein 2 | 28/45 | 2,124 ms |
| SymSpell deletion 1 | 15/45 | 0,001 ms |

Prefix-only gagal pada typo karena token yang salah ketik bukan prefix dari
token yang benar. Levenshtein 1 menemukan 28 dari 35 typo satu edit; Levenshtein
2 tidak menambah recall pada pemotongan 5 kandidat karena kandidat berfrekuensi
tinggi mendorong target keluar. SymSpell yang dibangun dari 50.000 token
teratas hanya menemukan 15/45 dan membutuhkan memori tambahan saat build.

## Keputusan

v1 hanya menjalankan exact match, lalu prefix match pada token. Tidak ada fuzzy
match default, tidak ada section format baru, dan tidak ada flag fuzzy yang
diumumkan di API.

Alasan:

1. Prefix-only mempertahankan presisi dan tidak menambah latensi.
2. Levenshtein 1 menambah recall typo, tetapi 17/45 query tetap gagal dan
   berpotensi menambah kandidat salah. Belum ada data presisi pada set yang
   lebih besar.
3. SymSpell deletion index menambah struktur build dan memori tanpa recall yang
   cukup pada data ini.
4. FR-005 meminta normalisasi dan alias, bukan fuzzy wajib. Alias dan varian
   nama sudah ditangani `alt_name`, `old_name`, `official_name`, `short_name`,
   dan `brand`.

Fuzzy Levenshtein 1 boleh ditambahkan di v2 sebagai flag eksplisit
`fuzzy=true` jika ada bukti pengguna. Implementasinya memakai token FST yang
sudah ada; tidak perlu menambah section atau mengubah format v1.

## Konsekuensi

Positif:

- Tidak ada false positive dari fuzzy pada default.
- Tidak ada biaya build, memori, atau format tambahan.
- Token FST tetap menjadi satu-satunya struktur pencarian nama.

Negatif:

- Typo satu huruf mengembalikan hasil kosong.
- Pengguna harus memperbaiki ejaan sendiri.
- Peluang peningkatan relevansi dari fuzzy belum dimanfaatkan.

## Alternatif yang ditolak

| Alternatif | Alasan |
|---|---|
| Levenshtein 1 default | Recall 28/45, presisi belum diukur, dan kandidat bertambah pada setiap query typo. |
| Levenshtein 2 default | Latensi 6 kali Levenshtein 1 tanpa tambahan recall pada top-5. |
| SymSpell deletion index | Recall 15/45 dan menambah struktur build yang harus divalidasi serta di-checksum. |
| Fuzzy selalu aktif | Melanggar ekspektasi exact match dan mempersulit evaluasi relevansi. |

## Tinjau ulang

Tinjau ulang jika ada 30 query typo nyata dari pengguna atau jika P4-03
menunjukkan top-1 turun karena ejaan. Ukur presisi dan recall pada set holdout
sebelum mengaktifkan flag.
