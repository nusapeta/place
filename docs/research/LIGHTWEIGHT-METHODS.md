# Metode ringan dan akurat untuk placeid

| Field | Value |
|---|---|
| Status | Draft riset |
| Tanggal | 2026-11-13 |
| Dokumen terkait | [TRD](../TRD.md), [BACKLOG](../BACKLOG.md), [INDEX-FORMAT](../INDEX-FORMAT.md), [BENCHMARKS](../BENCHMARKS.md) |

Dokumen ini menjawab satu pertanyaan: metode mana yang memberi akurasi cukup
untuk pencarian nama tempat Indonesia tanpa melampaui batas resource di
[SRS](../SRS.md). Fokusnya adalah metode yang bisa dibuktikan dengan
pengukuran, bukan daftar teknologi.

Batas yang mengikat:

| Batas | Nilai | Sumber |
|---|---|---|
| Search p95 | ≤ 50 ms pada 20 RPS, 4 vCPU | NFR-001 |
| Reverse p95 | ≤ 30 ms pada 20 RPS | NFR-002 |
| Index Indonesia | ≤ 600 MB | NFR-003 |
| Idle RSS | ≤ 50 MB | NFR-004 |
| Startup | ≤ 1 detik | NFR-005 |
| Build Indonesia | ≤ 15 menit, 12 worker, 16 GB | NFR-006 |
| Top-1 / top-5 | ≥ 80% / ≥ 95% pada 300 query | NFR-013 |

## 1. Ringkasan rekomendasi

| Area | Rekomendasi | Alasan | Task |
|---|---|---|---|
| Bentuk index | Satu file `mmap` dengan record fixed-size, FST, postings, dan grid | Memenuhi satu file, startup cepat, RSS idle kecil; didukung bukti GridPin dan open-geocode | P1-03, P1-11 |
| Forward search | Exact dan prefix lebih dulu, lalu irisan postings dengan token paling jarang | Menghindari scan token umum seperti `jalan`; prefix adalah jalur autocomplete alami | P2-02 |
| Ranking | Exact, prefix, cakupan token berbobot IDF, `importance`, proximity, tie-break deterministik | IDF sudah tersedia gratis dari `posting_count`; importance meniru sinyal Nominatim | P2-03 |
| Fuzzy | Prefix-only sebagai default; fuzzy di belakang flag dengan batas ketat, kandidat SymSpell deletion index jika perlu | Levenshtein automaton `fst` masih eksperimental dan boros memori; fuzzy puluhan hingga ratusan kali lebih lambat | P1-05 |
| Name duplikat | Name FST harus menunjuk postings, atau exact match memakai token postings | Satu nama bisa dipakai ribuan fitur, misalnya merek; value tunggal memotong recall | P1-06 |
| Reverse | Grid sparse 0,01 derajat dengan ring search dan haversine | 0,01 derajat sekitar 1,1 km di ekuator; ring search berhenti saat jarak terbaik lebih kecil dari batas ring berikutnya | P1-08, P2-04 |
| Admin | R-tree bbox dan point-in-polygon saat build, simpan `admin_id` saja | Polygon tidak perlu dibaca server; biaya ada di build | P3-04 |
| Build | Tiga pass streaming, set node referensi, sort eksplisit | Menghindari menyimpan 277 juta node dan menjaga determinisme | P1-02, P3-07 |
| Importance | Tag OSM lebih dulu; tabel Wikipedia hanya jika top-1 tidak tercapai | Wikipedia page rank adalah sinyal terkuat Nominatim, tetapi menambah data dan lisensi | P1-09 |
| Evaluasi | 300 query berlabel, split tune dan holdout | Tidak ada benchmark publik nama tempat Indonesia; tanpa label, tuning hanya menebak | P4-01 |

## 2. Bukti dari sistem yang sudah berjalan

Tiga sistem mendekati bentuk yang dituju. Angka di bawah diambil dari
dokumentasi publik mereka dan tidak diukur pada mesin placeid.

| Sistem | Bentuk | Angka publik | Pelajaran |
|---|---|---|---|
| GridPin | Satu file per negara, `mmap`, FST, sel spasial, typo automata | Prancis 365 MB untuk 26,1 juta alamat; exact sekitar 3.000 per detik per core; reverse sekitar 1.800 per detik; fuzzy 10 sampai 50 per detik dengan sekitar 60 ms | Satu file cukup untuk jutaan fitur; fuzzy adalah jalur lambat dan harus menjadi fallback |
| open-geocode | Tantivy, FST, Levenshtein automata, Hilbert R-tree, satu Pack | Belanda 0,36 GB; siap dalam 0,36 detik; p50 forward 0,72 ms, reverse 0,20 ms; puncak RAM 1,1 GiB saat melayani 16 vCPU | Tantivy dan FST bisa memenuhi latency; RAM puncak lebih besar dari target idle placeid, jadi target RSS harus diukur terpisah |
| Nominatim | PostgreSQL dan PostGIS, import planet berhari-hari | Importance dari page rank Wikipedia, fallback dari search rank | Kualitas ranking nama tempat banyak ditentukan importance, bukan hanya kecocokan teks |

Konsekuensi: bentuk satu file dan FST masuk akal. Target RSS 50 MB tidak boleh
disimpulkan dari angka open-geocode karena angka itu mencakup page cache dan
mesin 16 vCPU. Pengukuran sendiri tetap wajib.

## 3. Forward search

### 3.1 FST dan postings

`fst` menyimpan set atau map string besar secara kompak dan mendukung
pencarian prefix. Nilainya bisa berupa offset dan panjang postings, sehingga
satu lookup menghasilkan daftar row tanpa tabel hash per token.

Kelebihan:

- Ukuran kecil dan read-only, cocok untuk `mmap`.
- Prefix search adalah operasi bawaan, tidak perlu scan.
- Postings terurut sehingga irisan dua token linear dan bisa berhenti lebih
  awal.

Kekurangan:

- Immutable. Rebuild penuh, sesuai scope v1.
- Tidak menyimpan frekuensi token per dokumen. Ranking harus memakai sinyal
  lain, atau format ditambah.
- Levenshtein automaton bawaan masih eksperimental. Dokumentasi `fst`
  menyebut konstruksinya bisa memakai puluhan MB dan punya batas state.

Rekomendasi: pakai FST untuk token dan name lookup, tetapi jangan jadikan
Levenshtein automaton jalur default.

### 3.2 Query token umum

Query seperti `jalan` punya postings sangat panjang. TRD sudah menetapkan
strategi: ambil token paling jarang, lalu iris dengan token lain, dan batasi
kandidat 100.000 baris. Ini benar secara algoritmik dan murah.

Tambahan yang disarankan: hitung IDF dari `posting_count` yang sudah ada di
value FST. Tidak perlu format baru. Token seperti `pasar` atau `jalan`
mendapat bobot rendah, token seperti `beringharjo` mendapat bobot tinggi.
Ini memperbaiki ranking tanpa menambah section.

### 3.3 Prefix dan exact

Exact match harus dicek lebih dulu karena biayanya satu lookup. Namun exact
match tidak boleh berhenti pada satu row. Nama seperti `Indomaret` atau
`Malioboro` bisa dipakai banyak fitur. INDEX-FORMAT §10 saat ini menyimpan
satu row dengan importance tertinggi. Itu memotong recall dan membuat
`limit=5` tidak bisa mengembalikan lima cabang.

Dua opsi:

1. Ubah value name FST menjadi `(posting_offset << 32) | posting_count`,
   sama seperti token FST. Value tetap 64 bit, tetapi perlu satu daftar
   postings per nama unik, sekitar 5,1 MB untuk 1,26 juta fitur. Exact match
   dan prefix bisa mengembalikan semua kandidat.
2. Pertahankan name FST sebagai fast path satu row, lalu selalu jalankan
   token postings untuk melengkapi kandidat. Exact boost dihitung dengan
   membandingkan nama ternormalisasi di strings.

Rekomendasi: opsi 1 untuk format v1 karena lebih sederhana saat query dan
tidak bergantung pada perbandingan string untuk setiap kandidat. Keputusan
ini mengubah INDEX-FORMAT dan perlu ADR sebelum P1-11.

### 3.4 Engine alternatif

| Engine | Kekuatan | Biaya | Kapan dipilih |
|---|---|---|---|
| Custom FST | Ukuran kecil, kontrol penuh, tanpa dependency C, satu file | Parser dan ranking ditulis sendiri | Default jika gate ADR 0002 lulus |
| SQLite FTS5 | BM25, prefix, satu file, matang | Dependency C, query planner di luar kontrol, tidak punya grid dan admin | Jika custom index gagal gate ukuran atau waktu perawatan |
| Tantivy | Full-text matang, Levenshtein, segment dan BM25 | Multi-file, RAM lebih besar, tetap butuh grid dan admin sendiri | Jika ranking custom terlalu mahal dan footprint masih masuk |

ADR 0002 sudah menyebut SQLite sebagai fallback. Tantivy belum masuk gate
ADR. P1-04 harus mengukur ketiganya pada korpus nama yang sama. Jangan
memilih engine dari benchmark alamat negara lain.

## 4. Fuzzy matching

Fuzzy adalah fitur mahal. GridPin melaporkan fuzzy 10 sampai 50 query per
detik per core, sementara exact sekitar 3.000 per detik. Artinya fuzzy 60
sampai 300 kali lebih lambat pada jalur exact pendek, dan tidak boleh berjalan
pada setiap query.

Tiga strategi yang diuji:

| Strategi | Cara kerja | Risiko | Biaya query |
|---|---|---|---|
| Prefix-only | Token terakhir dicari sebagai prefix di token FST | Typo di tengah kata tidak tertangani | Sangat rendah |
| Levenshtein automaton `fst` | Automaton menyusuri FST dan menerima jarak edit tertentu | Dokumentasi menyebut eksperimental, konstruksi bisa puluhan MB, batas state | Rendah saat menyusuri, tinggi saat membangun automaton |
| SymSpell deletion index | Precompute varian delete untuk token; query mencari varian yang sama | Ukuran index bertambah; perlu batas panjang token dan jarak edit | Rendah, satu atau beberapa lookup |

Rekomendasi:

1. v1 default: exact, lalu prefix. Jika hasil kosong dan query memenuhi syarat
   (satu token, panjang ≥ 5, tanpa angka), coba fuzzy dengan batas kandidat
   dan waktu.
2. Jangan memakai `fst::automaton::Levenshtein` tanpa `new_with_limit` dan
   tanpa pengukuran. Jika dipakai, bangun automaton per token query, bukan per
   kandidat.
3. Jika 45 query typo dan alias di query set tidak tercapai, tambahkan
   SymSpell deletion index sebagai section opsional. Batasi jarak edit 1
   untuk token pendek dan 2 untuk token panjang.
4. Fuzzy tidak boleh mengubah urutan hasil exact. Beri penalti edit distance
   eksplisit.

## 5. Ranking

### 5.1 Sinyal yang murah

| Sinyal | Sumber | Biaya | Catatan |
|---|---|---|---|
| Exact | Name FST atau perbandingan string | Satu lookup | Bobot tertinggi |
| Prefix | Token FST | Satu range scan | Penting untuk autocomplete |
| Cakupan token | Jumlah token query yang cocok | Gratis saat irisan | Harus berbobot IDF |
| `importance` | Dihitung saat build | 4 byte per place | Meniru prior Nominatim |
| Proximity | `lat`/`lon` query | Satu haversine per kandidat | Hanya jika koordinat diberikan |
| Admin match | Parent place dari query | Opsional | Berguna untuk `Pasar Beringharjo Yogyakarta` |
| Language match | `accept-language` | Gratis | Memilih varian nama |

### 5.2 Importance

Nominatim memakai page rank Wikipedia sebagai nilai utama dan search rank
sebagai fallback. Untuk placeid, tabel Wikipedia per tempat berarti data
tambahan, proses join, dan pertanyaan lisensi. Alternatif ringan:

- Prior dari kelas fitur: admin dan `place` di atas `highway`, `highway` di
  atas POI, POI di atas bangunan.
- Bonus `population` dengan skala logaritmik.
- Bonus `capital`, `wikidata`, dan `brand`.
- Clamp 0 sampai 1.

Prior ini tidak sekuat page rank Wikipedia, tetapi cukup untuk membedakan
`Kota Bandung` dari `Bandung` di kabupaten lain. P1-09 harus mengukur selisih
top-1 dengan dan tanpa tabel Wikipedia pada 100 query. Jika selisihnya besar,
tabel eksternal menjadi kandidat ADR, bukan keputusan diam-diam.

### 5.3 Determinisme

Tie-break wajib: skor menurun, `importance` menurun, `place_id` menaik.
Tanpa ini, urutan hasil berubah antar query dan benchmark tidak bisa diulang.
Hindari reduksi `f32` paralel dengan urutan yang tidak tetap. Hitung skor per
kandidat secara independen, atau pakai penjumlahan berurutan yang deterministik.

### 5.4 Bobot

Bobot adalah konfigurasi server, bukan format index. Tuning dilakukan pada
split tune, dievaluasi pada holdout. Jangan mengubah bobot setelah melihat
holdout. Catat setiap perubahan bobot dan hasilnya di BENCHMARKS.

## 6. Reverse geocoding

### 6.1 Grid versus H3 versus R-tree

| Metode | Kelebihan | Kekurangan | Cocok untuk |
|---|---|---|---|
| Grid derajat tetap | Sederhana, tanpa dependency, sel bisa dihitung dengan integer | Sel tidak sama luas; kepadatan tidak merata | v1 Indonesia, radius 1 sampai 10 km |
| H3 | Sel heksagon, hierarki, bagus untuk agregasi dan sharding | Dependency, konversi sel, tidak perlu untuk satu negara | Setelah v1, multi-region |
| R-tree | Nearest neighbor efisien, bbox | Struktur pohon lebih besar dan lebih kompleks untuk dibaca dari `mmap` | Build-side admin, atau engine alternatif |

H3 resolusi 8 punya luas rata-rata 0,737 km² dan edge 0,53 km. Resolusi 7
punya luas 5,16 km². Grid 0,01 derajat di ekuator sekitar 1,11 km kali
1,11 km, jadi resolusi spasialnya sebanding dengan H3 resolusi 7 sampai 8.
Untuk Indonesia yang dekat ekuator, distorsi lintang kecil. Tidak ada alasan
menambah dependency H3 di v1.

### 6.2 Grid dense versus sparse

Bounding box Indonesia kira-kira 46 derajat bujur dan 17 derajat lintang.
Grid 0,01 derajat menghasilkan sekitar 7,9 juta sel. Dengan 8 byte per sel,
grid dense memakai sekitar 63 MB. Sebagian besar sel adalah laut atau tanpa
fitur.

Grid sparse menyimpan hanya sel yang terisi: kunci sel 32 bit, offset
postings 32 bit, dan count 32 bit, ditambah 4 byte per fitur di postings.
Pada 128.669 sel terukur, biayanya sekitar 1,5 MB untuk record sel dan 5,1 MB
untuk postings, total sekitar 6,6 MB. Jika setiap fitur menempati sel sendiri,
batas atasnya sekitar 20 MB. Grid dense memakai sekitar 63 MB untuk record
sel saja, ditambah 5,1 MB postings.

Rekomendasi: grid sparse dengan kunci sel terurut dan binary search.
INDEX-FORMAT §11 saat ini menetapkan grid dense. Perubahan ini perlu
keputusan sebelum format beku.

### 6.3 Ring search

Untuk radius R, jangan menghitung semua sel dalam kotak R. Mulai dari sel
titik, lalu perluas ring. Setelah ring ke-k selesai diproses, jarak minimum
ke titik mana pun di ring berikutnya adalah `k * cell_size`. Jika jarak fitur
terbaik sudah lebih kecil dari batas itu, berhenti. `cell_size` dikonversi ke
meter memakai nilai terkecil antara meter per derajat lintang dan bujur pada
lintang query, karena satu derajat bujur memendek menjauh dari ekuator. Ini
memberi hasil nearest yang benar selama sel menyimpan semua fitur dan jarak
dihitung dengan haversine.

Batas kandidat tetap perlu: satu sel di Jakarta bisa berisi ribuan fitur.
Urutkan kandidat per sel dengan jarak, atau simpan postings per sel terurut
berdasarkan jarak tidak mungkin karena sel tidak menyimpan koordinat. Solusi
sederhana: batas kandidat per query dan radius, lalu perluas ring hanya jika
kandidat belum cukup.

### 6.4 Akurasi reverse

Reverse placeid bukan reverse alamat. Yang dikembalikan adalah fitur bernama
terdekat. Secara konsep, untuk titik di jalan, jalan adalah jawaban yang
benar; secara implementasi, centroid jalan punya batas yang diukur di §6.5.
Untuk titik di dalam gedung, POI bisa lebih benar daripada jalan. Aturan
prioritas jenis di TRD (jarak selisih kurang dari 25 meter) adalah titik
tuning, bukan kontrak. P2-04 harus menguji:

- Titik di jalan utama.
- Titik di permukiman tanpa POI.
- Titik di laut.
- Titik tepat di batas sel.
- Titik dengan banyak POI dalam radius 50 meter.

### 6.5 Batas akurasi centroid jalan

Index menyimpan centroid way, bukan garis jalan. Untuk jalan panjang, titik di
ujung jalan bisa jauh dari centroid. Reverse dengan radius 1 km lalu
mengembalikan 404 atau fitur lain, bukan jalan yang tepat. Ini konsekuensi
model data, bukan bug parser.

Hasil ukur pada semua way bernama dengan `highway=*` (524.175 way, scan
2026-11-13 dengan lokasi node):

| Metrik | p50 | p90 | p99 | Maksimum |
|---|---:|---:|---:|---:|
| Diagonal bbox way | 107 m | 610 m | 4,19 km | 76,9 km |
| Jarak node terjauh dari rata-rata koordinat node | 60 m | 344 m | 2,31 km | 39,9 km |

Sebanyak 31.861 way (6,1%) punya diagonal di atas 1 km dan 991 way (0,19%)
di atas 10 km. Artinya sekitar 90% jalan punya semua node dalam 344 m dari
pusat, sehingga radius default 1 km cukup. Sekitar 1% jalan punya bagian
yang lebih dari 2,3 km dari pusat, dan bagian itu bisa keluar dari radius.

Pilihan:

1. Pertahankan centroid untuk v1, dokumentasikan bahwa reverse tidak akurat
   untuk tepi jalan panjang, dan ukur dengan label reverse yang memuat jalan
   panjang.
2. Simpan bbox per way, sekitar 8,4 MB untuk 524.175 way, sebagai filter
   kasar. Bbox memperkecil kandidat, tetapi tidak menghitung jarak ke garis,
   jadi tidak memperbaiki kasus jalan panjang.
3. Simpan polyline sederhana atau postings segmen per sel. Ini menambah
   ukuran dan kompleksitas query, dan hanya layak jika target akurasi reverse
   gagal.

Rekomendasi: opsi 1 untuk v1. Jangan mengklaim reverse tingkat alamat atau
tepi jalan. P2-04 dan query reverse di BENCHMARKS harus memuat jalan panjang
sebagai kasus batas.

## 7. Hierarki admin

Polygon admin tidak dibutuhkan server. Build menghitung `admin_id` terdalam
untuk setiap place, lalu server hanya membaca rantai parent. Ini menjaga
index tetap kecil dan query tetap cepat.

Algoritma build:

1. Kumpulkan relation `boundary=administrative` level 4 sampai 7 dengan nama.
2. Susun ring dari member way role `outer` dan `inner`.
3. Buang ring tidak tertutup atau kurang dari 3 titik, catat jumlahnya.
4. Hitung bbox dan luas.
5. Bangun R-tree bbox.
6. Untuk setiap place, ambil kandidat polygon yang mengandung titik.
7. Pilih polygon dengan luas terkecil; jika sama, `admin_id` terkecil.
8. Simpan `admin_id`; susun parent dari level dan containment.

Kebutuhan memori build: jumlah polygon admin kecil dibanding fitur, tetapi
vertex bisa jutaan. R-tree dan ray casting hanya berjalan saat build, jadi
tidak menambah RSS server.

Jumlah terukur pada scan 2026-11-13: 9.405 relation `boundary=administrative`
level 4 sampai 7, 2 di antaranya tanpa nama. Rincian per level: 60 level 4,
597 level 5, 2.197 level 6, dan 6.551 level 7. Total way member 136.737,
maksimum 547 per relation. Jumlah record admin di index kira-kira 0,38 MB,
bukan 0,04 MB seperti perkiraan awal. Vertex polygon adalah biaya build,
bukan biaya index server, karena polygon tidak perlu dibaca saat melayani
query.

Risiko: polygon OSM bisa self-intersect atau memiliki lubang tidak valid.
Kebijakan TRD adalah melewati dan mencatat, bukan memaksa. Ini benar. Jangan
menggagalkan seluruh build karena satu polygon.

## 8. Build dan memori

PBF Indonesia berisi 328,2 juta objek, 277,2 juta di antaranya node. Jangan
menyimpan semua node.

Rencana tiga pass:

1. Pass 1: kumpulkan fitur bernama, tag, dan referensi node way/relation.
2. Pass 2: simpan koordinat hanya untuk node ID yang dirujuk.
3. Pass 3: resolusi geometri way dan relation, lalu tulis index.

Ukuran set referensi perlu diukur (P1-01, P1-02). Perkiraan kasar: way
bernama sekitar 878 ribu, rata-rata 10 node, berarti sekitar 9 juta referensi.
Node ID 8 byte dan koordinat 8 byte, sekitar 150 MB. Ini masih di bawah 16 GB,
tetapi peak RSS parser PBF dan struktur sementara harus diukur, bukan
diasumsikan.

Alternatif jika memori ketat: external sort node referensi ke disk, lalu
merge. Ini menambah I/O dan waktu. Jangan dipilih sebelum P1-02 menunjukkan
kebutuhan.

Determinisme:

- Sort fitur berdasarkan `place_id`.
- Sort postings menaik dan dedup.
- Sort token secara leksikografis sebelum FST.
- Sort varian nama berdasarkan `lang` lalu string.
- Jangan iterasi `HashMap` untuk menulis output.
- Hash index body dua kali dan bandingkan.

## 9. Normalisasi nama Indonesia

Aturan SRS dan TRD sudah cukup untuk v1: NFKC, lowercase, hapus diakritik,
tanda baca menjadi spasi, dan pemetaan `Jl.`, `Kec.`, `Kab.`, `Kel.`, `Kp.`,
`No.`.

Tambahan yang layak diuji:

- `Jln.` dan `Jalan` (varian umum).
- `Gg.` dan `Gang`.
- `Ds.` dan `Desa`.
- `Kecamatan` vs `Kec.` sudah tercakup.
- Nama dengan apostrof dan tanda hubung.
- Angka romawi dan angka biasa pada nama jalan.
- Gelar dan singkatan pada nama POI, misalnya `RS`, `RSUD`, `SMP`, `SD`.
- Nama merek dengan spasi dan tanpa spasi.

Jangan menambahkan stemmer bahasa Indonesia di v1. Stemmer bisa mengubah nama
tempat dan merek, dan biaya evaluasinya lebih besar daripada manfaatnya.
Alias OSM memberi hasil yang lebih aman.

## 10. Model ukuran index

Angka di bawah diukur pada 2026-11-13 dari PBF Indonesia yang sama dengan
README. Pemindaian memakai `pyosmium` dengan filter `name`, sehingga hanya
objek bernama yang diproses. Waktu scan 22,6 detik. Pemindaian geometri jalan
(524.175 way `highway=*` bernama) memakai lokasi node dan berjalan 52,5 detik.
Pemindaian admin (9.405 relation level 4 sampai 7) berjalan 4,6 detik.

| Besaran | Hasil terukur | Catatan |
|---|---:|---|
| Fitur bernama | 1.263.949 | 346.846 node, 878.076 way, 39.027 relation |
| Teks `name` | 21.243.045 B | 21,2 MB, sebelum dedup |
| Nama ternormalisasi unik | 612.877 | README menyebut 645.238 nama mentah |
| Nama ternormalisasi duplikat | 129.914 | Dipakai lebih dari satu fitur |
| Occurrence token | 3.453.904 | Termasuk token dari varian nama |
| Token unik | 184.938 | Rata-rata 7,16 byte per token |
| Teks varian nama | 1.972.346 B | 118.348 tag varian |
| Sel grid 0,01 derajat | 128.669 | Node bernama saja, lower bound |
| Sel grid 0,02 derajat | 82.515 | Node bernama saja |
| Sel grid 0,05 derajat | 33.867 | Node bernama saja |

Nama duplikat terbanyak: `indomaret` 4.141 fitur, `alfamart` 3.591,
`linha de agua` 2.192, `toko` 1.964, `blok` 1.875. Ini membuktikan bahwa
name FST dengan satu nilai per nama tidak cukup untuk query merek.

Token terbanyak: `jalan` 430.290, `gang` 82.445, `raya` 39.776,
`masjid` 36.583, `1` 25.242. IDF dari `posting_count` akan menurunkan bobot
token-token ini secara otomatis.

Ukuran FST diukur dengan crate `fst` 0.4.7 pada token dan nama asli:

| FST | Key | Ukuran file | Waktu build |
|---|---:|---:|---:|
| Token FST, value `u64` | 184.938 | 1.187.384 B (1,2 MB) | 60 ms |
| Name FST, value `u64` | 612.877 | 6.692.328 B (6,7 MB) | 458 ms |

Model ukuran index v1:

| Section | Rumus | Ukuran | Catatan |
|---|---|---:|---|
| Places | 1.263.949 × 64 B | 80,9 MB | Fixed-size record |
| Strings nama | 21.243.045 B | 21,2 MB | Upper bound tanpa dedup |
| Strings varian | 1.972.346 B | 2,0 MB | Terukur |
| Record varian | 118.348 × 12 B | 1,4 MB | Terukur |
| Token FST | file FST | 1,2 MB | Terukur |
| Postings | 3.453.904 × 4 B | 13,8 MB | `u32` |
| Name FST | file FST | 6,7 MB | Terukur |
| Name postings, opsi 1 | 1.263.949 × 4 B | 5,1 MB | Hanya jika name FST menunjuk postings |
| Grid sparse, record | 128.669 × 12 B | 1,5 MB | Lower bound |
| Grid sparse, postings | 1.263.949 × 4 B | 5,1 MB | Satu row per fitur |
| Admin | 9.403 × 40 B | 0,38 MB | Jumlah terukur, ukuran record perkiraan |
| Header dan trailer | 256 B + 32 B | kecil | |
| **Total tanpa polygon, opsi 1** | | **sekitar 139 MB** | |
| **Total tanpa polygon, name FST satu row** | | **sekitar 134 MB** | Format saat ini |
| Grid dense sebagai pembanding | 7,88 juta × 8 B + postings | sekitar 200 MB | Tanpa name postings |

Total 139 MB memberi margin sekitar 4,3 kali terhadap batas 600 MB. Bahkan
jika grid sparse terisi satu sel per fitur (1.263.949 sel), totalnya sekitar
153 MB. Angka ini belum termasuk padding antar section, string admin, dan
polygon admin. Polygon admin tetap opsional dan bisa dikeluarkan dari v1.

Dua catatan metodologi:

- Occupancy grid dihitung dari node bernama saja karena way tidak di-resolve
  ke koordinat node pada pass ini. Fitur way dan relation akan menambah sel
  terisi, tetapi tidak mungkin melebihi jumlah fitur.
- Nama unik 612.877 lebih kecil dari 645.238 karena normalisasi menggabungkan
  perbedaan huruf besar, diakritik, dan tanda baca, serta membuang token satu
  karakter non-digit. Angka ini yang relevan untuk key FST.

Batas 600 MB tidak terancam oleh keputusan format yang sudah diketahui.
Risiko ukuran justru ada di polygon admin dan di pilihan dense grid. Keduanya
sudah masuk spike P1-03 dan P1-08.

## 11. Evaluasi akurasi

Tidak ada benchmark publik untuk pencarian nama tempat Indonesia. Benchmark
yang ada mengukur alamat, bukan nama tempat. Karena itu NFR-013 harus diuji
dengan query set sendiri.

Protokol:

- 300 query, stratifikasi sesuai BENCHMARKS §4.
- Setiap query punya `osm_type` dan `osm_id` atau daftar fitur yang diterima.
- Label dibuat sebelum tuning, disimpan di repo sebagai data uji.
- Split 200 tune dan 100 holdout. Holdout tidak dipakai untuk memilih bobot.
- Laporkan top-1, top-5, empty rate per kategori.
- Untuk reverse, laporkan jarak ke fitur yang diharapkan.
- Jangan menyimpulkan dari agregat jika satu kategori turun.

Risiko overfitting: 300 query cukup untuk mengukur, tidak cukup untuk
melatih model besar. Karena itu jangan memakai ML di v1. Tuning bobot
dengan grid search kecil dan holdout lebih jujur.

## 12. Yang tidak dipilih untuk v1

| Metode | Alasan |
|---|---|
| ML ranking | Data label terlalu kecil; biaya inferensi dan pemeliharaan tidak sepadan |
| Embedding vector | Index dan RAM membesar; tidak jelas lebih akurat untuk nama tempat |
| Stemmer bahasa Indonesia | Mengubah nama diri dan merek; alias OSM lebih aman |
| H3 | Tidak perlu untuk satu negara; grid derajat cukup |
| R-tree di server | Grid sparse lebih kecil dan cukup untuk radius 10 km |
| Tantivy sebagai default | Multi-file dan RAM lebih besar; tetap kandidat jika gate custom gagal |
| Polygon admin di server | Server hanya butuh `admin_id`; polygon hanya untuk build dan audit |
| Wikipedia importance sebagai default | Data dan lisensi tambahan; ukur dulu manfaatnya |

## 13. Pertanyaan yang harus dijawab spike

| Pertanyaan | Task | Keputusan jika jawabannya buruk |
|---|---|---|
| Berapa token unik dan occurrences sebenarnya? | P1-01 | Sesuaikan model postings dan FST |
| Berapa peak RSS dan waktu dua pass PBF? | P1-02 | External sort atau kurangi pass |
| Apakah grid sparse cukup untuk radius 10 km? | P1-08 | Grid dua tingkat atau R-tree build-side |
| Apakah name FST postings menambah ukuran signifikan? | P1-06 | Token-only exact match |
| Apakah fuzzy diperlukan untuk mencapai NFR-013? | P1-05 | Prefix-only, atau SymSpell terbatas |
| Apakah importance OSM cukup? | P1-09 | Tabel Wikipedia dengan ADR |
| Apakah custom index lulus gate ADR 0002? | P1-04 | SQLite FTS5 atau Tantivy |
| Apakah akurasi reverse jalan cukup dengan centroid? | P2-04, P4-05 | Bbox per way atau geometri polyline |

## 14. Sumber

- `fst` crate, dokumentasi dan peringatan Levenshtein automaton:
  https://docs.rs/fst/latest/fst/automaton/struct.Levenshtein.html
- `fst` crate, ringkasan desain hemat memori: https://docs.rs/fst
- GridPin, dokumentasi dan angka throughput:
  https://gridpin.dev/docs dan https://gridpin.dev/bench
- open-geocode, arsitektur dan benchmark:
  https://github.com/open-geocode-rs/open-geocode dan https://opengeocode.com/docs/
- Nominatim, place ranking:
  https://nominatim.org/release-docs/develop/customize/Ranking/
- Nominatim, importance:
  https://nominatim.org/release-docs/develop/customize/Importance/
- H3, tabel resolusi dan luas sel: https://h3geo.org/docs/core-library/restable/
- SymSpell, algoritma symmetric delete:
  https://github.com/wolfgarbe/SymSpell
- SQLite FTS5, dokumentasi: https://sqlite.org/fts5.html
- Tantivy, dokumentasi arsitektur: https://docs.rs/tantivy
