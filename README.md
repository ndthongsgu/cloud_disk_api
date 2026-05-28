# Cloud Disk API Backend

Dự án **"Nghiên cứu và phát triển nền tảng lưu trữ đám mây sử dụng Telegram Bot API"** (Cloud Disk) là hệ thống lưu trữ vô hạn tận dụng Telegram Bot API làm node lưu trữ vật lý, phân tách dữ liệu thành các file chunk và quản lý bằng metadata chặt chẽ dựa trên cơ chế phân quyền ReBAC linh hoạt.

---

## 🛠️ Yêu cầu Hệ thống
Trước khi bắt đầu, hãy đảm bảo máy tính của bạn đã cài đặt các công cụ sau:
1. **Rust Toolchain**: Phiên bản ổn định mới nhất (hỗ trợ Rust Edition 2024).
2. **Docker & Docker Compose**: Để khởi chạy PostgreSQL Database container.
3. **SQLx CLI**: Công cụ quản lý cơ sở dữ liệu và migrations của SQLx.

---

## 🚀 Các bước khởi động hệ thống

### Bước 1: Thiết lập cấu hình biến môi trường
Tạo hoặc kiểm tra tệp tin `.env` tại thư mục gốc của dự án. Đảm bảo cấu hình khớp với thông tin kết nối cơ sở dữ liệu:
```env
PORT=8080
DATABASE_URL=postgresql://cloud_disk:cloud_disk@localhost:5433/cloud_disk
JWT_SECRET=my_secret_key
```

---

### Bước 2: Khởi động Cơ sở dữ liệu (PostgreSQL)
Hệ thống sử dụng Docker Compose để chạy cơ sở dữ liệu PostgreSQL 15 trên cổng `5433` (để tránh xung đột với PostgreSQL mặc định chạy cổng 5432 trên máy bạn).

Chạy lệnh sau để khởi động cơ sở dữ liệu ở chế độ nền (background):
```bash
docker compose up -d
```

Để kiểm tra trạng thái hoạt động của Database:
```bash
docker compose ps
```

---

### Bước 3: Cài đặt SQLx CLI (Nếu chưa có)
Cài đặt `sqlx-cli` phiên bản hỗ trợ **PostgreSQL** bằng công cụ `cargo` của Rust:
```bash
cargo install sqlx-cli --no-default-features --features postgres
```

---

### Bước 4: Khởi tạo Database và chạy Migrations thủ công
Dự án được cấu hình để chạy migrations **thủ công** qua dòng lệnh. 

1. **Khởi tạo cơ sở dữ liệu (nếu chưa có) và áp dụng tất cả các migrations**:
   Lệnh này sẽ tự động đọc cấu hình `DATABASE_URL` trong file `.env`, tạo database `cloud_disk` và chạy toàn bộ mã SQL trong thư mục `migrations/`:
   ```bash
   sqlx database setup
   ```

2. **Hoặc chỉ chạy các file migrations mới (nếu database đã tồn tại)**:
   ```bash
   sqlx migrate run
   ```

3. **Để kiểm tra trạng thái của các migrations**:
   ```bash
   sqlx migrate info
   ```

4. **Để hoàn tác (revert) migrations gần nhất**:
   ```bash
   sqlx migrate revert
   ```

---

### Bước 5: Chạy ứng dụng Backend (Actix-Web Server)
Sau khi database đã được cấu hình và chạy migrations thành công, bạn khởi chạy ứng dụng backend:
```bash
cargo run
```

Server sẽ khởi động tại địa chỉ: `http://127.0.0.1:8080`

---

## 🧪 Kiểm tra API (Endpoints)

Bạn có thể sử dụng `curl`, Postman hoặc trình duyệt để kiểm tra hoạt động của API:

1. **API Hello World (Kiểm tra Server chạy)**:
   ```bash
   curl http://127.0.0.1:8080/
   ```
   *Kết quả mong đợi:* `Hello Actix-Web!`

2. **API Get Users (Kiểm tra truy vấn Database)**:
   ```bash
   curl "http://127.0.0.1:8080/users?page=1&size=10"
   ```

---

## 📁 Cấu trúc cơ sở dữ liệu (Database Schema)
Các bảng dữ liệu được tạo ra trong migration `0001_init.sql` bao gồm:
- **IAM**: `users`, `roles`, `permissions`, `user_roles`, `role_permissions`, `employees`, `customers`.
- **ReBAC Security**: `relationships` (Hệ thống lưu trữ các mối quan hệ phân quyền linh hoạt theo dạng `subject` -> `relation` -> `object`).
- **Core Domain**:
  - `telegram_bots`: Pool các con bot làm node lưu trữ vật lý nhằm tối ưu Rate Limit.
  - `folders`: Cấu trúc cây thư mục ảo vô hạn cấp của người dùng.
  - `files`: Metadata tệp tin logic người dùng nhìn thấy.
  - `video_metadata`: Siêu dữ liệu bổ trợ tối ưu hóa phát/stream video.
  - `file_chunks`: Trái tim của cơ chế Streaming phân mảnh tệp tin lớn (được đánh composite index `idx_file_chunks_lookup` để tua video cực nhanh).
