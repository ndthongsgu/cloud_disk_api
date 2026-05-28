use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trạng thái tiến trình tải lên và xử lý tệp tin
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UploadStatus {
    /// Đang chờ phân mảnh/xử lý khởi tạo luồng
    Pending,
    /// Đang tiến hành truyền tải dữ liệu chunk lên Telegram
    Uploading,
    /// Đang tối ưu hóa định dạng, nén hoặc quét mã
    Processing,
    /// Quá trình tải lên đã hoàn tất thành công
    Completed,
    /// Xảy ra sự cố lỗi trong quá trình tải dữ liệu
    Failed,
}

/// Mô hình quản lý thông tin tệp tin logic của người dùng
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct File {
    /// ID duy nhất của tệp tin logic
    pub id: Uuid,
    /// Ngày tải lên tệp tin
    pub created_date: NaiveDateTime,
    /// ID người tạo/tải lên tệp tin
    pub created_by: Option<Uuid>,
    /// Ngày cập nhật thông tin tệp gần nhất
    pub updated_date: NaiveDateTime,
    /// ID người cập nhật tệp gần nhất
    pub updated_by: Option<Uuid>,
    /// Phiên bản tệp phục vụ khóa lạc quan (Optimistic Locking)
    pub version: i32,
    /// Trạng thái xóa mềm (Soft delete)
    pub is_deleted: bool,
    /// Mã tệp tin duy nhất
    pub code: String,
    /// ID tài khoản người dùng sở hữu tệp (khóa ngoại trỏ đến users.id)
    pub user_id: Uuid,
    /// ID thư mục cha chứa tệp ảo này (NULL đại diện cho thư mục gốc)
    pub folder_id: Option<Uuid>,
    /// Tên gốc của tệp tin lúc tải lên (ví dụ: movie.mp4)
    pub file_name: String,
    /// Định dạng tài nguyên MIME-Type (ví dụ: video/mp4, image/jpeg)
    pub mime_type: String,
    /// Dung lượng tổng thể của tệp gốc (byte)
    pub size_bytes: i64,
    /// Đánh dấu nhanh tệp thuộc kiểu video giúp hệ thống định tuyến luồng phát trực tiếp
    pub is_video: bool,
    /// Tiến trình tải lên hiện tại của tệp
    pub upload_status: UploadStatus,
}
