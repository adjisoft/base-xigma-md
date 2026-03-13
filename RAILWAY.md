# Panduan Deploy Xigma-MD ke Railway

Dokumen ini menjelaskan langkah lengkap menjalankan Xigma-MD di Railway menggunakan Dockerfile yang sudah ada.

## Ringkasannya
- Railway akan membangun image dari `Dockerfile`.
- Bot tidak perlu port HTTP, jadi jalankan sebagai service/worker.
- Session WhatsApp sebaiknya disimpan di volume agar login tidak hilang saat redeploy.

## 1) Prasyarat
- Repo ini sudah di-push ke GitHub (atau Git provider lain yang didukung Railway).
- `config.ron` sudah diisi sesuai kebutuhan.
- Kamu paham method login yang dipakai (`pairing` atau `qrcode`).

## 2) Persiapan `config.ron`
Pastikan nilai di `config.ron` valid, terutama:
- `NO_OWNER`, `NAMA_OWNER`, `NAMA_BOT`
- `METHOD_LOGIN`: rekomendasi `qrcode` untuk Railway (lihat catatan login di bawah)
- `BOT_MODE`: `public` atau `self`

Catatan:
- Aplikasi membaca `config.ron` di direktori kerja `/app` dalam container.
- File ini juga bisa berubah saat runtime (mis. lewat command `/set`), tapi perubahan akan hilang jika tidak memakai volume.

## 3) Deploy ke Railway (Dockerfile)
Langkah umum (tanpa tergantung nama menu UI):
1. Buat project baru di Railway.
2. Tambahkan service dari repo Git yang berisi project ini.
3. Pastikan Railway memakai `Dockerfile` untuk build (bukan Nixpacks).
4. Jalankan deploy.

Jika Railway meminta port:
- Pilih mode service non-web/worker, atau nonaktifkan healthcheck berbasis HTTP.
- Bot ini berjalan sebagai proses background dan tidak membuka port HTTP.

## 4) Tambahkan Volume (Sangat Disarankan)
Agar session login tidak hilang saat restart/redeploy:
- Tambahkan volume dan mount ke `/app/session`.

Opsional:
- Tambahkan volume kedua ke `/app/message_debugs` jika ingin menyimpan hasil `/debug`.

Tanpa volume:
- Bot akan meminta login ulang setelah container dibangun ulang.

## 5) Proses Login WhatsApp di Railway
### A. Jika `METHOD_LOGIN = "qrcode"`
- Setelah deploy, buka log Railway.
- QR code akan tampil sebagai ASCII di log.
- Scan QR dari WhatsApp untuk menyelesaikan login.

### B. Jika `METHOD_LOGIN = "pairing"`
- Aplikasi akan membaca nomor dari `session/phone.txt`.
- Jika file itu tidak ada, aplikasi akan menunggu input dari STDIN, dan ini biasanya membuat bot "stuck" di Railway.
- Solusi:
  - Buat `session/phone.txt` lebih dulu (isi nomor format internasional, contoh `628xxxx`).
  - Pastikan `session/phone.txt` tersimpan di volume `/app/session`.

## 6) Verifikasi Bot Berjalan
Cek log Railway dan pastikan ada pesan:
- `✅ Berhasil login sebagai: ...`
- `🚀 Bot berjalan...`

Jika tidak muncul, cek bagian troubleshooting.

## 7) Update & Redeploy
- Push perubahan ke repo.
- Redeploy service di Railway.
- Pastikan volume `/app/session` tetap sama agar tidak login ulang.

## 8) Troubleshooting
### Build gagal di Railway
Bagian **Known Issues** di `readme.md` menyebutkan status build per 5 Maret 2026.
Jika build masih gagal, cek bagian tersebut lalu perbaiki error yang tercantum sebelum redeploy.

### Bot berhenti di prompt "Masukan nomer WhatsApp"
Penyebab:
- `METHOD_LOGIN = pairing` dan `session/phone.txt` belum ada.

Solusi:
- Ganti `METHOD_LOGIN` ke `qrcode`, atau
- Buat `session/phone.txt` lebih dulu di volume `/app/session`.

### Login ulang setiap redeploy
Penyebab:
- Volume `/app/session` belum dipasang.

Solusi:
- Tambahkan volume dan mount ke `/app/session`.

### `yt-dlp` atau `ffmpeg` tidak ditemukan
Dockerfile sudah meng-install `yt-dlp` dan `ffmpeg`.
Jika error masih muncul, rebuild image dan cek log build Railway.

## 9) Struktur File yang Relevan di Container
- `/app/config.ron`
- `/app/session/bot.db`
- `/app/session/phone.txt`
- `/app/message_debugs/`

## 10) Catatan Keamanan
- Hindari menyimpan data sensitif di repo publik.
- Jika repo publik, simpan `config.ron` secara private atau gunakan repo privat.
