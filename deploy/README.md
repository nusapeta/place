# Deploy placeid

Satu container, satu index Indonesia, read-only. Index tidak dibangun di host
tujuan: builder butuh RAM ~8,2 GB, sedangkan homelab hanya 7,8 GB total.
Build dilakukan di mesin lain, lalu hanya artefak rilis yang dikirim.

| Artefak | Ukuran | Keterangan |
|---|---|---|
| `places.idx` | 141.997.032 B | Index mmap format v1 |
| `places.idx.json` | 1.417 B | Manifest: checksum isi, jumlah, bbox, PBF asal |
| `places.idx.sha256` | 77 B | Checksum file untuk `sha256sum -c` |

## 1. Kirim artefak dan image

```bash
# di mesin build
sha256sum places.idx > places.idx.sha256
ssh arifsani@192.168.1.219 'mkdir -p ~/Projects/placeid/{data,deploy}'
rsync -av places.idx places.idx.json places.idx.sha256 \
  arifsani@192.168.1.219:~/Projects/placeid/data/
docker save placeid:0.1 | gzip -1 | ssh arifsani@192.168.1.219 'gunzip | docker load'

# di host tujuan: checksum wajib sama dengan manifest
cd ~/Projects/placeid && sha256sum data/places.idx
```

Salin juga `docker-compose.yml`, `docker-compose.homelab.yml`, dan `.env` ke
`~/Projects/placeid/deploy/`. Jangan menyalin direktori `data/` lewat git.

## 2. Jalankan

```bash
cd ~/Projects/placeid/deploy
docker compose up -d          # .env sudah memuat COMPOSE_FILE
docker compose ps
docker logs --tail 20 placeid
```

`.env` berisi `COMPOSE_FILE=docker-compose.yml:docker-compose.homelab.yml`.
Override itu menambahkan service ke jaringan `1panel-network` dengan alias
`placeid`, jaringan yang sama dengan container `cloudflared-tunnel` milik
1Panel. Tanpa override, ingress tunnel tidak bisa resolve origin.

Compose project diberi nama eksplisit `placeid` (`name:` di bagian atas).
Tanpa itu Compose memakai nama direktori `deploy`, sama dengan project routeid
di host ini.

Port host hanya loopback: `127.0.0.1:18092`. Port `8090` di host sudah dipakai
container `nawasena-pb`; `18090`/`18091`/`18093` juga sudah terpakai.

## 3. Verifikasi

```bash
curl -fsS 127.0.0.1:18092/healthz
curl -sS '127.0.0.1:18092/search?q=monas&limit=2'
curl -sS '127.0.0.1:18092/reverse?lat=-6.175392&lon=106.827153'
curl -sS '127.0.0.1:18092/lookup?osm_ids=N1308678148'
docker inspect placeid --format '{{.State.Health.Status}}'   # healthy
```

`/healthz` mengembalikan `manifest_sha256` isi index. Nilai itu harus sama
dengan `index.sha256` di manifest; `--verify-checksum` membuat start gagal
kalau file tidak cocok.

Dari luar, lewat bundle gateway:

```bash
for p in /place/healthz '/place/search?q=monas&limit=2' \
         '/place/reverse?lat=-6.175392&lon=106.827153' \
         '/place/lookup?osm_ids=N1308678148'; do
  printf '%-46s ' "$p"
  curl -s -o /dev/null -w '%{http_code}\n' "https://route.arifsani.xyz$p"   # 200
done
curl -s -o /dev/null -w '%{http_code}\n' https://route.arifsani.xyz/place/metrics   # 404
```

## 4. Akses publik

Semua jalur publik melewati Cloudflare, karena ingress 80/443 dari internet
tidak bisa dijangkau langsung: router tidak menjawab UPnP maupun NAT-PMP, dan
probe eksternal ke `82.158.129.136:80`/`:443` berakhir `Connection timed out`
(pola yang sama dengan `home.arifsani.xyz`, yang DNS-only dan juga tidak
terjangkau). Jadi jangan pilih mode DNS-only untuk `place`.

### 4.1 Bundle lewat gateway routeid (jalur yang dipakai)

Ingress tunnel mencocokkan **hostname asli**, dan `place.arifsani.xyz` tidak
punya rule di sana. Yang punya rule adalah `route.arifsani.xyz` menuju
`gateway:8080`, jadi geocoder dipasang sebagai prefix di gateway itu:

| Path publik | Diteruskan ke |
|---|---|
| `https://route.arifsani.xyz/route` | `routeid` / `routeid-motor` |
| `https://route.arifsani.xyz/place/*` | `http://placeid:8090` |

```bash
curl -s 'https://route.arifsani.xyz/place/healthz'
curl -s 'https://route.arifsani.xyz/place/search?q=monas&limit=2'
curl -s 'https://route.arifsani.xyz/place/reverse?lat=-6.175392&lon=106.827153'
curl -s 'https://route.arifsani.xyz/place/lookup?osm_ids=N1308678148'
```

Tidak ada perubahan di repo ini untuk jalur tersebut: cukup container
`placeid` hidup di `1panel-network` dengan alias `placeid`. Sisi gateway
(flag `-place`, kuota, penutupan `/place/metrics`) didokumentasikan di
`deploy/README.md` repo routeid.

Batas yang berlaku pada prefix itu:

| Aturan | Nilai |
|---|---|
| Kuota per IP | 240/menit, burst 40, dijawab `429` + `Retry-After` |
| `/place/metrics` | `404` (`-block-metrics` di gateway) |
| `/place` saja | `404`, upstream tidak punya halaman index |
| `placeid` mati | `/place/*` menjawab `502`; `/healthz` gateway tetap `200` |
| Path yang dilihat upstream | prefix dibuang, jadi `/place/search` menjadi `/search` |

### 4.2 Hostname khusus (belum aktif)

`place.arifsani.xyz` sudah punya CNAME proxied ke tunnel
`af067798-9776-40b1-b4f3-1245741c710d`, tetapi tunnel itu tidak punya rule
untuk hostname tersebut, jadi jawabannya `404`. Untuk mengaktifkannya:

1. Salin vhost [`openresty/place.arifsani.xyz.conf`](openresty/place.arifsani.xyz.conf)
   ke `/opt/1panel/apps/openresty/openresty/conf/conf.d/` (milik `root`; tanpa
   sudo gunakan `docker cp` ke container 1Panel openresty), lalu
   `nginx -t && nginx -s reload`. Vhost itu menutup `/metrics` dengan `404` dan
   memberi `limit_req` 20 r/s per IP.
2. Di dashboard Cloudflare: **Networking > Tunnels > af067798-... > Routes >
   Add route > Published application**, hostname `place.arifsani.xyz`, service
   URL `http://192.168.1.219:80`. Origin harus IP host (openresty), bukan
   `placeid:8090`, supaya `/metrics` tetap tertutup vhost, bukan oleh aplikasi.

Langkah 2 butuh dashboard atau token dengan `Cloudflare Tunnel: Edit`. Token
yang tersedia saat deploy hanya punya `DNS: Edit`, dan seluruh endpoint
konfigurasi tunnel menolak (`GET`/`PUT .../cfd_tunnel/{tid}/configurations` →
`401` kode `1001`, `POST /cfd_tunnel` → `403` kode `10000`).

Jalur lain yang **sudah dicoba dan tidak bekerja**, jangan diulang:

| Upaya | Hasil |
|---|---|
| CNAME berantai ke hostname lain yang sudah ada di ingress | `404`: ingress dicocokkan pada hostname asli, bukan tujuan CNAME |
| CNAME ke hostname Cloudflare Quick Tunnel (`*.trycloudflare.com`) | `403` `error code: 1014`: CNAME lintas pengguna dilarang |
| Quick tunnel dengan `--http-host-header place.arifsani.xyz` | hanya berlaku pada hostname acak `*.trycloudflare.com` itu sendiri |
| Port forward manual di router | tidak ada UPnP/NAT-PMP; inbound juga tidak dirutekan ISP |

### 4.3 Jangan menerbitkan `/metrics`

`/metrics` selalu terbit dan memuat ukuran, umur, dan jumlah fitur index. Dua
jalur publik di atas menutupnya dengan cara berbeda, dan keduanya sudah diuji:

| Jalur | Yang menutup |
|---|---|
| Bundle gateway routeid | flag `-block-metrics` di gateway (`/place/metrics` → `404`) |
| Hostname khusus lewat openresty | vhost `place.arifsani.xyz.conf` (`location = /metrics` → `404`) |

Kalau nanti ditambah jalur ketiga, tutup juga di situ; jangan mengandalkan
aplikasi, karena server memang tidak pernah menyembunyikan endpoint itu.
Monitoring tetap membaca `http://127.0.0.1:18092/metrics` langsung.

## 5. Update index

Index immutable per proses. Rilis baru berarti container baru:

```bash
rsync -av places.idx places.idx.json places.idx.sha256 \
  arifsani@192.168.1.219:~/Projects/placeid/data/
ssh arifsani@192.168.1.219 \
  'cd ~/Projects/placeid/deploy && docker compose up -d --force-recreate'
```

Hentikan dulu, jangan menimpa file di bawah proses yang sedang memetakannya.
`docker compose restart` tidak cukup: file yang sudah termap tidak ikut berubah.

## 6. Rollback

```bash
docker compose down
# kembalikan places.idx + places.idx.json versi sebelumnya, lalu
docker compose up -d
```

`--verify-checksum` menolak pasangan index/manifest yang tidak cocok, jadi
rollback setengah jalan akan gagal start dengan pesan checksum, bukan menyajikan
data salah.

## 7. Batas sumber daya

| Batas | Nilai | Alasan |
|---|---|---|
| `mem_limit` | 256m | Index 142 MB dipetakan read-only; halaman file bisa direklaim cgroup |
| `pids_limit` | 128 | Satu proses Tokio, tanpa fork |
| `read_only` | ya | Server tidak menulis apa pun |
| `cap_drop` | ALL | Tidak butuh capability |
| Log | 10 MB x 3 | `PLACEID_LOG_FORMAT=json` |
