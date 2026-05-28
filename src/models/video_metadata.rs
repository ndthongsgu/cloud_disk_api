use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Mô hình lưu trữ thông tin kỹ thuật bổ trợ riêng cho Video (Quan hệ 1:1 với File)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct VideoMetadata {
    /// ID duy nhất của siêu dữ liệu video
    pub id: Uuid,
    /// Ngày khởi tạo thông tin video
    pub created_date: NaiveDateTime,
    /// ID người khởi tạo
    pub created_by: Option<Uuid>,
    /// Ngày cập nhật thông tin video gần nhất
    pub updated_date: NaiveDateTime,
    /// ID người cập nhật gần nhất
    pub updated_by: Option<Uuid>,
    /// Phiên bản bản ghi phục vụ khóa lạc quan (Optimistic Locking)
    pub version: i32,
    /// Trạng thái xóa mềm (Soft delete)
    pub is_deleted: bool,
    /// Mã siêu dữ liệu video
    pub code: String,
    /// ID tệp tin gốc sở hữu video (khóa ngoại trỏ đến files.id)
    pub file_id: Uuid,
    /// Tổng thời lượng trình chiếu của video (tính theo giây)
    pub duration_seconds: i32,
    /// Độ phân giải hiển thị video (ví dụ: 1080p, 4K, 720p)
    pub resolution: Option<String>,
    /// Codec mã hóa hình ảnh hỗ trợ play trên trình duyệt (ví dụ: h264, h265, vp9)
    pub video_codec: Option<String>,
    /// Codec mã hóa âm thanh đi kèm (ví dụ: aac, mp3, opus)
    pub audio_codec: Option<String>,
    /// Đánh dấu video có ảnh đại diện preview hay không
    pub has_thumbnail: bool,
    /// ID file lưu trữ ảnh thumbnail trên Telegram Server để hiển thị nhanh
    pub thumbnail_telegram_file_id: Option<String>,
}
