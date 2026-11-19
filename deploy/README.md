# Deploy Nusapeta Place

Satu container, satu index Indonesia, read-only. Index tidak dibangun di host
tujuan: builder butuh RAM sekitar 8 GiB. Build di mesin lain, lalu kirim
artefaknya.

| Artefak | Ukuran | Keterangan |
|---|---|---|
| `places.idx` | 386.376.512 B | Index mmap format v2 (pembaca juga menerima v1) |
| `places.idx.json` | 1.424 B | Manifest: checksum isi, jumlah, bbox, PBF asal |
| `places.idx.sha256` | 81 B | Checksum file untuk `sha256sum -c` |

## 1. Siapkan artefak

```bash
# di mesin build
sha256sum places.idx > places.idx.sha256

# di host tujuan
mkdir -p data
rsync -av places.idx places.idx.json places.idx.sha256 host:/srv/place/data/
sha256sum -c data/places.idx.sha256
```

Nilai `sha256` isi index juga dicatat di manifest sebagai `index.sha256`.
Server membandingkannya saat start dengan `--verify-checksum`.

## 2. Jalankan

```bash
cd deploy
cp .env.example .env
docker compose pull          # atau: docker build -t nusapeta-place:0.1 ..
docker compose up -d
docker compose ps
docker logs --tail 20 nusapeta-place
```

`.env` memuat `NUSA_PLACE_IMAGE`, `NUSA_PLACE_PORT`, dan `COMPOSE_FILE`.
Port host hanya loopback; jangan menerbitkan `8090` langsung ke internet.
Container berjalan sebagai UID 65532 dengan root filesystem read-only.

Untuk host yang sudah punya container tunnel di jaringan Docker sendiri,
pakai override `examples/homelab/docker-compose.override.yml` dan set
`TUNNEL_NETWORK` ke nama jaringan itu.

## 3. Verifikasi

```bash
curl -fsS 127.0.0.1:8090/healthz
curl -sS '127.0.0.1:8090/search?q=monas&limit=2'
curl -sS '127.0.0.1:8090/reverse?lat=-6.175392&lon=106.827153'
curl -sS '127.0.0.1:8090/lookup?osm_ids=N1308678148'
docker inspect nusapeta-place --format '{{.State.Health.Status}}'   # healthy
```

`/healthz` mengembalikan `manifest_sha256` isi index. Nilai itu harus sama
dengan `index.sha256` di manifest; `--verify-checksum` membuat start gagal
kalau file tidak cocok.

## 4. Akses publik

Place tidak menangani TLS, autentikasi, atau rate limit. Pilih satu jalur:

**Reverse proxy.** Salin `examples/openresty/place.conf.example`, ganti
`place.example.com` dan path sertifikat, lalu reload. Contoh itu menutup
`/metrics` dengan 404 dan memberi `limit_req` 20 r/s per IP.

**Prefix di gateway route.** Kalau Nusapeta Route dijalankan dengan
`nusa-gateway -place http://nusapeta-place:8090`, geocoder tersedia di bawah
`/place/*` pada base URL route. Gateway membuang prefix sebelum meneruskan,
menerapkan kuota per IP, dan menutup `/place/metrics`.

Jangan menerbitkan `/metrics`. Isinya ukuran, umur, dan jumlah fitur index;
tidak ada kredensial di sana, tetapi ukuran internal tidak perlu publik. Kalau
menambah jalur publik baru, tutup juga di jalur itu. Monitoring membaca
`http://127.0.0.1:8090/metrics` langsung.

## 5. Update index

Index immutable per proses. Rilis data baru berarti container baru:

```bash
rsync -av places.idx places.idx.json places.idx.sha256 host:/srv/place/data/
ssh host 'cd /srv/place/deploy && docker compose up -d --force-recreate'
```

Hentikan dulu, jangan menimpa file di bawah proses yang sedang memetakannya.
`docker compose restart` tidak cukup: file yang sudah termap tidak ikut berubah.

## 6. Rollback

```bash
docker compose down
# kembalikan places.idx + places.idx.json versi sebelumnya
docker compose up -d
```

`--verify-checksum` menolak pasangan index/manifest yang tidak cocok, jadi
rollback setengah jalan gagal start dengan pesan checksum, bukan menyajikan
data salah.

## 7. Batas sumber daya

| Batas | Nilai | Alasan |
|---|---|---|
| `mem_limit` | 256m | Index 386 MB dipetakan read-only; halaman file bisa direklaim cgroup |
| `pids_limit` | 128 | Satu proses Tokio, tanpa fork |
| `read_only` | ya | Tidak ada state yang perlu ditulis |
| `cap_drop` | ALL | Tidak ada operasi privileged |

Angka RSS dan latency ada di [../docs/BENCHMARKS.md](../docs/BENCHMARKS.md).
Panduan operasi lengkap ada di [../docs/OPERATIONS.md](../docs/OPERATIONS.md).
