use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Mô hình quản lý thông tin các mảnh phân đoạn vật lý của file tải lên Telegram (1:N với File)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct FileChunk {
    /// ID duy nhất của mảnh file
    pub id: Uuid,
    /// Ngày tải lên mảnh tệp tin này
    pub created_date: NaiveDateTime,
    /// ID khách hàng/hệ thống tải mảnh tệp tin
    pub created_by: Option<Uuid>,
    /// Ngày cập nhật mảnh tệp tin gần nhất
    pub updated_date: NaiveDateTime,
    /// ID người cập nhật gần nhất
    pub updated_by: Option<Uuid>,
    /// Phiên bản bản ghi phục vụ khóa lạc quan (Optimistic Locking)
    pub version: i32,
    /// Trạng thái xóa mềm (Soft delete)
    pub is_deleted: bool,
    /// Mã mảnh phân đoạn tệp tin
    pub code: String,
    /// ID tệp tin gốc sở hữu mảnh phân đoạn này (khóa ngoại trỏ đến files.id)
    pub file_id: Uuid,
    /// ID của Telegram Bot quản lý và lưu giữ mảnh file này (khóa ngoại trỏ đến telegram_bots.id)
    pub bot_id: Uuid,
    /// Vị trí thứ tự chỉ số của mảnh tệp tin gốc (bắt đầu từ 0)
    pub chunk_index: i32,
    /// Vị trí byte bắt đầu của phân đoạn trong file gốc (ví dụ: 0)
    pub start_byte: i64,
    /// Vị trí byte kết thúc của phân đoạn trong file gốc (ví dụ: 10485760)
    pub end_byte: i64,
    /// Dung lượng thực tế của mảnh phân đoạn (byte)
    pub chunk_size_bytes: i64,
    /// ID tệp tin duy nhất do Telegram API cấp để tải tệp trực tiếp
    pub telegram_file_id: String,
    /// ID tin nhắn chứa tệp tin trên kênh chat của Telegram Bot
    pub telegram_message_id: i64,
}
