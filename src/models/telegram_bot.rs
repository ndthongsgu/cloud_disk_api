use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trạng thái hoạt động của Telegram Bot dùng làm node lưu trữ vật lý
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum BotStatus {
    /// Bot hoạt động bình thường
    Active,
    /// Bị dính giới hạn tần suất gửi tin từ API Telegram (Rate Limit)
    RateLimited,
    /// Bị khóa/cấm bởi Telegram
    Banned,
}

/// Mô hình quản lý Telegram Bot làm phân đoạn node lưu trữ dữ liệu
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct TelegramBot {
    /// ID duy nhất của Bot
    pub id: Uuid,
    /// Ngày Bot được tích hợp vào hệ thống
    pub created_date: NaiveDateTime,
    /// ID người thêm Bot
    pub created_by: Option<Uuid>,
    /// Ngày cập nhật trạng thái hoạt động gần nhất
    pub updated_date: NaiveDateTime,
    /// ID người cập nhật gần nhất
    pub updated_by: Option<Uuid>,
    /// Phiên bản bản ghi phục vụ khóa lạc quan (Optimistic Locking)
    pub version: i32,
    /// Trạng thái xóa mềm (Soft delete)
    pub is_deleted: bool,
    /// Mã định danh bot duy nhất
    pub code: String,
    /// Tên Username định danh của Bot trên Telegram (ví dụ: MyDiskBot)
    pub bot_username: String,
    /// Chuỗi mã Token API kết nối của Bot (Cần được mã hóa ở mức ứng dụng)
    pub bot_token: String,
    /// Trạng thái kết nối của Bot
    pub status: BotStatus,
    /// Tổng số lượng file chunk vật lý mà bot này đã/đang lưu giữ
    pub total_files_handled: i64,
}
