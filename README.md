# Drive-RS

Google Drive benzeri dosya yönetimi ve ToffeShare benzeri P2P doğrudan dosya transferi uygulaması.

## Teknolojiler

### Backend
- **Rust** (edition 2024)
- **Axum** - Web framework
- **Tokio** - Async runtime
- **PostgreSQL** + SQLx - Veritabanı
- **WebSocket** - P2P signaling sunucusu

### Frontend
- **Svelte 5** + **SvelteKit**
- **WebRTC** - Doğrudan P2P dosya transferi

## Özellikler

- Kullanıcı kayıt ve giriş (JWT authentication)
- Dosya yükleme, indirme, silme, yeniden adlandırma
- Klasör oluşturma, silme, navigasyon
- Dosya paylaşım linkleri (süre sınırlı, şifreli)
- Sürükle-bırak dosya yükleme
- Depolama alanı takibi
- **P2P Dosya Transferi** - Dosyalar sunucuda depolanmadan doğrudan cihazdan cihaza aktarılır (WebRTC)

## Kurulum

### 1. PostgreSQL

```bash
docker compose up -d
```

### 2. Backend

```bash
cd backend
cp .env.example .env
# .env dosyasını düzenleyin
cargo run
```

Backend `http://localhost:3000` adresinde çalışacak.

### 3. Frontend

```bash
cd frontend
npm install
npm run dev
```

Frontend `http://localhost:5173` adresinde çalışacak (API istekleri backend'e proxy edilir).

## API Endpoints

### Auth
- `POST /api/auth/register` - Kayıt
- `POST /api/auth/login` - Giriş
- `GET /api/auth/me` - Kullanıcı bilgisi

### Dosyalar
- `GET /api/files?folder_id=` - Dosya listesi
- `POST /api/files/upload?folder_id=` - Dosya yükle (multipart)
- `GET /api/files/:id` - Dosya indir
- `DELETE /api/files/:id` - Dosya sil
- `PATCH /api/files/:id/rename` - Yeniden adlandır
- `PUT /api/files/:id/move` - Taşı

### Klasörler
- `GET /api/folders?folder_id=` - Klasör listesi
- `POST /api/folders` - Klasör oluştur
- `GET /api/folders/:id` - Klasör bilgisi
- `DELETE /api/folders/:id` - Klasör sil
- `PATCH /api/folders/:id/rename` - Yeniden adlandır

### Paylaşım
- `POST /api/share` - Paylaşım linki oluştur
- `GET /api/share/:token` - Paylaşım bilgisi
- `GET /api/share/:token/download` - Paylaşılan dosyayı indir

### P2P Transfer
- `POST /api/transfer` - Transfer oturumu oluştur
- `GET /api/transfer/:token` - Oturum bilgisi
- `WS /api/transfer/ws/:token` - WebRTC signaling WebSocket
