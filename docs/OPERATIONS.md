# Operasi placeid

| Field | Value |
|---|---|
| Status | Draft, prosedur target |
| Tanggal | 2026-11-13 |
| Dokumen terkait | [SRS](SRS.md), [API](API.md), [BENCHMARKS](BENCHMARKS.md) |

Dokumen ini adalah runbook untuk build, deploy, rebuild, dan rollback. Perintah
CLI sudah tersedia di binary rilis. Angka kapasitas dan latensi adalah target
sampai benchmark penuh dicatat di [BENCHMARKS](BENCHMARKS.md).

## 1. Prasyarat

| Kebutuhan | Nilai |
|---|---|
| OS | Linux amd64 atau arm64 |
| Rust | Stable, versi dicatat di manifest |
| PBF | Extract Indonesia, file `.osm.pbf` |
| Disk build | Minimal 3 GB bebas untuk PBF, index, dan file sementara |
| RAM build | 16 GB |
| RAM serve | 512 MB sampai 1 GB, target idle ≤ 50 MB |
| Port | 8090, hanya di jaringan internal |

Jangan menyimpan PBF atau index di git.

## 2. Build index

### 2.1 Build dari source

```bash
cargo build --release
```

Binary yang dihasilkan:

```text
target/release/placeid-build
target/release/placeid-serve
```

### 2.2 Jalankan builder

```bash
mkdir -p /srv/placeid/releases/2026-11

/usr/bin/time -v target/release/placeid-build \
  --input /data/indonesia-latest.osm.pbf \
  --output /srv/placeid/releases/2026-11/places.idx \
  --manifest /srv/placeid/releases/2026-11/places.idx.json \
  --verify
```

Builder menulis ke file sementara lalu mengganti nama secara atomik. Jika
proses gagal, index lama tidak tersentuh.

### 2.3 Verifikasi

```bash
ls -lh /srv/placeid/releases/2026-11/
jq '.format_version, .counts, .index.sha256' \
  /srv/placeid/releases/2026-11/places.idx.json
```

Verifikasi checksum penuh dengan menjalankan server sementara:

```bash
target/release/placeid-serve \
  --index /srv/placeid/releases/2026-11/places.idx \
  --manifest /srv/placeid/releases/2026-11/places.idx.json \
  --verify-checksum \
  --bind 127.0.0.1:18090
```

Di terminal lain:

```bash
curl -fsS http://127.0.0.1:18090/healthz
curl -fsS http://127.0.0.1:18090/version
```

Hentikan server sementara setelah kedua endpoint merespons 200.

## 3. Deploy

### 3.1 Container

```bash
docker build -t placeid:0.1 .

docker run --rm \
  --name placeid \
  --read-only \
  --user 65532:65532 \
  -p 127.0.0.1:8090:8090 \
  -v /srv/placeid/current:/data:ro \
  placeid:0.1 \
  --index /data/places.idx \
  --manifest /data/places.idx.json \
  --bind 0.0.0.0:8090
```

Aturan container:

- Root filesystem read-only.
- User non-root.
- Volume index read-only.
- Tidak ada koneksi keluar.
- Batas memori 512 MB.
- Healthcheck memanggil `/healthz`.

### 3.1a Deploy homelab (Compose + tunnel 1Panel)

Host `arifsani-pc1` memakai Compose, bukan `docker run`. File lengkap ada di
[`deploy/README.md`](../deploy/README.md); ringkasnya:

| Item | Nilai |
|---|---|
| Project Compose | `placeid` (`name:` eksplisit) |
| Port host | `127.0.0.1:18092` (loopback, `8090` dipakai `nawasena-pb`) |
| Alias jaringan | `placeid` di `1panel-network` (jaringan `cloudflared-tunnel`) |
| Batas memori | `256m`, `pids_limit` 128, `cap_drop: ALL` |
| Artifak | `~/Projects/placeid/data/{places.idx,places.idx.json,places.idx.sha256}` |

```bash
cd ~/Projects/placeid/deploy
docker compose up -d            # .env memuat COMPOSE_FILE dengan override homelab
curl -fsS 127.0.0.1:18092/healthz
```

Index dibangun di mesin lain (puncak RSS 8,2 GB) lalu dikirim sebagai artefak;
image dipindahkan dengan `docker save | ssh ... docker load`. Jangan pernah
membangun index di host dengan 7,8 GB RAM.

Akses publik memakai bundle gateway `routeid`, karena ingress tunnel
mencocokkan hostname asli dan hanya `route.arifsani.xyz` yang punya rule:

| Path publik | Diteruskan ke |
|---|---|
| `https://route.arifsani.xyz/place/*` | `http://placeid:8090` |
| `https://route.arifsani.xyz/route` | `routeid` / `routeid-motor` |

Gateway itu membuang prefix sebelum meneruskan, jadi kontrak `/search`,
`/reverse`, `/lookup`, dan `/healthz` tidak berubah. Healthcheck gateway tetap
hanya `/healthz` milik routing: `placeid` yang mati membuat `/place/*`
menjawab `502` tanpa membuat gateway dianggap gagal.

Dua hal yang tidak boleh dilepas:

- `/metrics` wajib ditutup di jalur publik, karena endpoint itu selalu aktif
  dan memuat ukuran serta umur index. Bundle menutupnya dengan
  `-block-metrics` di gateway; vhost openresty menutupnya sendiri.
- Kuota per IP untuk `/place` (240/menit, burst 40) menjaga agar satu klien
  tidak menghabiskan CPU host: pencarian memindai seluruh index dan plafon
  terukurnya sekitar 20 RPS.

Hostname khusus `place.arifsani.xyz` belum aktif: CNAME-nya sudah ada, tetapi
tunnel belum punya rule untuk hostname itu sehingga jawabannya `404`. Prosedur
dan jalur yang sudah terbukti gagal ada di
[`deploy/README.md`](../deploy/README.md) §4.2.

### 3.2 systemd

```ini
[Unit]
Description=placeid geocoder
After=network-online.target

[Service]
ExecStart=/usr/local/bin/placeid-serve \
  --index /srv/placeid/current/places.idx \
  --manifest /srv/placeid/current/places.idx.json \
  --bind 127.0.0.1:8090
DynamicUser=yes
ProtectSystem=strict
ProtectHome=yes
PrivateTmp=yes
NoNewPrivileges=yes
ReadOnlyPaths=/srv/placeid
MemoryMax=512M
Restart=on-failure
RestartSec=2

[Install]
WantedBy=multi-user.target
```

## 4. Konfigurasi

| Environment | Flag | Default | Catatan |
|---|---|---|---|
| `RUST_LOG` | tidak ada | `info` | Filter `tracing`, misalnya `placeid=debug` |
| `PLACEID_LOG_FORMAT` | tidak ada | teks | `json` menghasilkan satu objek per baris |
| `PLACEID_WEIGHT_EXACT` | tidak ada | `3.0` | Bobot ranker |
| `PLACEID_WEIGHT_PREFIX` | tidak ada | `1.5` | Bobot ranker |
| `PLACEID_WEIGHT_COVERAGE` | tidak ada | `1.0` | Bobot ranker |
| `PLACEID_WEIGHT_IMPORTANCE` | tidak ada | `0.5` | Bobot ranker |
| `PLACEID_WEIGHT_PROXIMITY` | tidak ada | `0.5` | Bobot ranker |

Checksum index divalidasi saat start hanya jika `--verify-checksum`
diberikan. Flag CLI mengalahkan nilai default; environment hanya mengatur
filter log, format log, dan bobot ranker. Jangan menaruh nilai rahasia di
environment; layanan tidak butuh rahasia.

## 5. Health dan monitoring

| Endpoint | Frekuensi | Aksi |
|---|---|---|
| `/healthz` | 10 detik | Restart jika 503 lebih dari 3 kali |
| `/version` | Saat deploy | Cocokkan checksum manifest |
| `/metrics` | 30 detik | Scrape Prometheus |

Retensi metrik: Prometheus menyimpan 15 hari, resolusi scrape 30 detik.
Histogram memakai bucket default detik. Jangan menambahkan label query,
koordinat, atau `place_id` ke metrik; kardinalitas dan privasi dilarang.
Arsip agregat harian disimpan 90 hari di luar Prometheus. Dashboard ada di
`ops/grafana-placeid.json`, alert di `ops/prometheus-alerts.yml`.

Alert minimum:

- `placeid_requests_total{status=~"5.."}` lebih dari 1% selama 10 menit.
- p95 latency di atas target selama 10 menit.
- Working set container di atas 480 MiB selama 10 menit, mendekati batas container 512 MB. Angka ini mencakup page cache index; beban 20 RPS terukur 150.180 KiB, sedangkan idle 6,1 MiB. NFR-004 mengikat RSS idle, bukan working set saat beban.
- `/healthz` gagal 3 kali berturut-turut.
- Index lebih tua dari 45 hari.
- Build terakhir gagal.

## 6. Rebuild

Rebuild penuh, bukan update diff. Prosedur untuk dua replika:

1. Build ke direktori rilis baru.
2. Verifikasi manifest dan checksum.
3. Jalankan server sementara di port lain dengan index baru.
4. Panggil `/healthz`, `/version`, dan beberapa query asap.
5. Arahkan satu replika ke rilis baru, restart.
6. Ulangi untuk replika kedua.
7. Simpan rilis sebelumnya untuk rollback.

Jika hanya satu replika, hentikan server, ganti symlink `current`, lalu start.
Downtime singkat tidak bisa dihindari pada mode satu replika.

Promote dan rollback otomatis memakai symlink atomik:

```bash
scripts/promote_release.sh /srv/placeid/releases/2026-11 /srv/placeid
systemctl restart placeid
scripts/rollback_release.sh /srv/placeid
systemctl restart placeid
```

`promote_release.sh` menolak rilis tanpa `places.idx`, manifest, atau
`places.idx.sha256`; membandingkan SHA-256 file index; memastikan
`format_version == 1` dan `counts.features > 0`; menyimpan target sebelumnya
di `previous`; lalu mengganti `current` dengan `mv -Tf`. Rollback menolak
`previous` yang hilang atau tidak lengkap. Keduanya tidak me-restart service;
restart tetap langkah eksplisit agar operator melihat downtime.

### 6.1 Runbook rebuild penuh

1. Pastikan disk untuk output minimal 250 MB dan RAM minimal 10 GB.
2. Jalankan build ke direktori rilis baru, bukan menimpa `current`:

   ```bash
   python3 scripts/measure_build.py target/release/placeid-build \
     --input /data/indonesia-latest.osm.pbf \
     --output /srv/placeid/releases/2026-11/places.idx \
     --manifest /srv/placeid/releases/2026-11/places.idx.json \
     --verify
   ```

3. Buat checksum file dan periksa manifest:

   ```bash
   sha256sum /srv/placeid/releases/2026-11/places.idx \
     > /srv/placeid/releases/2026-11/places.idx.sha256
   jq '.format_version, .counts.features, .counts.skipped_features' \
     /srv/placeid/releases/2026-11/places.idx.json
   ```

4. Jalankan server sementara pada port lain dengan `--verify-checksum`.
5. Panggil `/healthz`, `/version`, satu search, satu reverse, satu lookup.
6. Promosikan dengan `scripts/promote_release.sh`, restart, lalu ulangi smoke
   pada port layanan.
7. Simpan rilis sebelumnya minimal dua versi untuk rollback.
8. Catat wall time, peak RSS, ukuran, checksum, dan jumlah fitur ke
   [BENCHMARKS](BENCHMARKS.md).

Build yang gagal di tengah tidak boleh menyentuh `current`. Builder menulis
output dan manifest baru; verifikasi checksum dilakukan sebelum promote.

## 7. Rollback

```bash
ln -sfn /srv/placeid/releases/2026-10 /srv/placeid/current
systemctl restart placeid
curl -fsS http://127.0.0.1:8090/version
```

Rollback tidak memerlukan rebuild. Index sebelumnya harus masih ada. Simpan
minimal dua rilis terakhir.

## 8. Backup dan retensi

- Index bisa dibangun ulang dari PBF dan builder yang sama. Backup utama
  adalah PBF, manifest, dan versi builder.
- Simpan minimal dua index terakhir untuk rollback.
- Jangan menyimpan PBF di git. Gunakan object storage atau disk terpisah.
- Manifest menyimpan SHA-256 PBF. Tanpa PBF yang cocok, reproduksibilitas
  tidak bisa diverifikasi.
- Log dan metrik mengikuti kebijakan operator. Layanan tidak menyimpan query.

## 9. Kapasitas

Target awal adalah 20 RPS pada 4 vCPU. Untuk menambah kapasitas:

- Tambah replika dengan index yang sama.
- Pastikan file index dibaca dari storage lokal atau page cache yang cukup.
- Jangan menaruh index di NFS dengan latency tinggi.
- Pantau CPU, RSS, dan p95. Jika CPU di atas 70% pada beban puncak, tambah
  replika.

Satu index bisa dipakai beberapa proses read-only. Tidak ada koordinasi
antar proses.

## 10. Troubleshooting

| Gejala | Kemungkinan | Tindakan |
|---|---|---|
| `/healthz` 503 | Index belum termap atau header salah | Cek path, permission, dan log start |
| Server exit saat start | Magic, versi, atau checksum salah | Jalankan `--verify-checksum`, cek manifest |
| p95 naik | Kandidat terlalu banyak atau CPU penuh | Cek metrik kandidat, tambah replika, audit query umum |
| RSS naik terus | Page cache index atau kebocoran | Cek `smaps_rollup`, bandingkan setelah idle |
| Build OOM | Pass menyimpan terlalu banyak referensi | Turunkan worker, cek jumlah node ref, tambah RAM |
| Build lambat | Disk lambat atau polygon admin | Cek waktu per pass, matikan polygon opsional |
| Checksum mismatch | File berubah atau build tidak selesai | Build ulang, jangan pakai file tersebut |
| Permission denied | Volume read-only atau user salah | Cek UID container dan mode file |
| Port sudah dipakai | Proses lain | Ganti `--bind` atau hentikan proses lama |
| Hasil kosong | Index salah atau query terlalu ketat | Uji dengan nama yang diketahui, cek manifest |

## 11. Keamanan operasi

- Jalankan sebagai user non-root.
- Root filesystem read-only.
- Tidak ada shell di image produksi.
- Tidak ada koneksi keluar dari server.
- Batasi port di firewall atau gateway.
- Rate limit di gateway, bukan di `placeid`.
- Audit dependency dengan `cargo audit` sebelum rilis.
- Jangan menaruh token atau kredensial di manifest.

## 12. Checklist rilis

- [ ] Semua test lulus.
- [ ] `cargo audit` bersih atau temuan dicatat.
- [ ] Build deterministik diverifikasi.
- [ ] Benchmark dicatat di [BENCHMARKS](BENCHMARKS.md).
- [ ] Manifest memuat PBF SHA, index SHA, dan versi builder.
- [ ] Smoke test `/search`, `/reverse`, `/lookup`, `/healthz`.
- [ ] Rollback ke rilis sebelumnya diuji.
- [ ] Atribusi ODbL terlihat di respons.
