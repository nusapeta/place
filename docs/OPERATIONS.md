# Operasi placeid

| Field | Value |
|---|---|
| Status | Draft, prosedur target |
| Tanggal | 2026-11-13 |
| Dokumen terkait | [SRS](SRS.md), [API](API.md), [BENCHMARKS](BENCHMARKS.md) |

Dokumen ini adalah runbook untuk build, deploy, rebuild, dan rollback. Perintah
CLI adalah kontrak target v1. Kode belum ada, jadi perintah belum bisa
dijalankan sampai fase 1 selesai.

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
  --pbf /data/indonesia-latest.osm.pbf \
  --out /srv/placeid/releases/2026-11/places.idx \
  --manifest /srv/placeid/releases/2026-11/places.idx.json \
  --workers 12
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
  --verify-checksum \
  --addr 127.0.0.1:18090
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
  --addr 0.0.0.0:8090
```

Aturan container:

- Root filesystem read-only.
- User non-root.
- Volume index read-only.
- Tidak ada koneksi keluar.
- Batas memori 512 MB.
- Healthcheck memanggil `/healthz`.

### 3.2 systemd

```ini
[Unit]
Description=placeid geocoder
After=network-online.target

[Service]
ExecStart=/usr/local/bin/placeid-serve \
  --index /srv/placeid/current/places.idx \
  --addr 127.0.0.1:8090
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
| `PLACEID_INDEX` | `--index` | tidak ada | Wajib |
| `PLACEID_ADDR` | `--addr` | `127.0.0.1:8090` | Alamat listen |
| `PLACEID_LOG` | `--log` | `info` | `error`, `warn`, `info`, `debug` |
| `PLACEID_MAX_CONCURRENCY` | `--max-concurrency` | jumlah vCPU | Batas request berat |
| `PLACEID_VERIFY_CHECKSUM` | `--verify-checksum` | `false` | Verifikasi penuh saat start |

Flag mengalahkan environment. Jangan menaruh nilai rahasia di environment;
layanan tidak butuh rahasia.

## 5. Health dan monitoring

| Endpoint | Frekuensi | Aksi |
|---|---|---|
| `/healthz` | 10 detik | Restart jika 503 lebih dari 3 kali |
| `/version` | Saat deploy | Cocokkan checksum manifest |
| `/metrics` | 30 detik | Scrape Prometheus |

Alert minimum:

- `placeid_requests_total{status=~"5.."}` lebih dari 1% selama 10 menit.
- p95 latency di atas target selama 10 menit.
- RSS di atas 80 MB selama 10 menit.
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

```bash
ln -sfn /srv/placeid/releases/2026-11 /srv/placeid/current
systemctl restart placeid
```

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
| Port sudah dipakai | Proses lain | Ganti `--addr` atau hentikan proses lama |
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
