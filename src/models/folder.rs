use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Mô hình quản lý thư mục ảo của người dùng (Hỗ trợ cấu trúc cây vô cấp)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Folder {
    /// ID duy nhất của thư mục ảo
    pub id: Uuid,
    /// Ngày tạo thư mục
    pub created_date: NaiveDateTime,
    /// ID khách hàng tạo thư mục
    pub created_by: Option<Uuid>,
    /// Ngày cập nhật thư mục gần nhất
    pub updated_date: NaiveDateTime,
    /// ID người cập nhật gần nhất
    pub updated_by: Option<Uuid>,
    /// Phiên bản thư mục phục vụ khóa lạc quan (Optimistic Locking)
    pub version: i32,
    /// Trạng thái xóa mềm (Soft delete)
    pub is_deleted: bool,
    /// Mã thư mục ảo duy nhất
    pub code: String,
    /// ID tài khoản người dùng sở hữu thư mục (khóa ngoại trỏ đến users.id)
    pub user_id: Uuid,
    /// ID của bot lưu thư mục này
    pub bot_id: Uuid,
    /// ID thư mục cha (NULL đại diện cho thư mục gốc - Root Folder)
    pub parent_folder_id: Option<Uuid>,
    /// Tên hiển thị của thư mục (ví dụ: Tài liệu)
    pub folder_name: String,
    /// Mã màu tùy chỉnh hiển thị cho thư mục ảo trên giao diện người dùng
    pub color_code: Option<String>,
}
