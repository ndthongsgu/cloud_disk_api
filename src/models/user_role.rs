use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Mô hình liên kết nhiều-nhiều giữa User và Role
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct UserRole {
    /// ID duy nhất của bản ghi liên kết
    pub id: Uuid,
    /// Ngày thiết lập vai trò cho người dùng
    pub created_date: NaiveDateTime,
    /// ID người gán vai trò này
    pub created_by: Option<Uuid>,
    /// Ngày cập nhật liên kết gần nhất
    pub updated_date: NaiveDateTime,
    /// ID người cập nhật liên kết gần nhất
    pub updated_by: Option<Uuid>,
    /// Phiên bản bản ghi phục vụ khóa lạc quan (Optimistic Locking)
    pub version: i32,
    /// Trạng thái xóa mềm (Soft delete)
    pub is_deleted: bool,
    /// ID người dùng sở hữu vai trò
    pub user_id: Uuid,
    /// ID vai trò được gán cho người dùng
    pub role_id: Uuid,
}
