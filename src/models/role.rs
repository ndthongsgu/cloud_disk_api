use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Mô hình quản lý vai trò của người dùng trong hệ thống (RBAC)
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct Role {
    /// ID duy nhất của vai trò
    pub id: Uuid,
    /// Ngày tạo vai trò
    pub created_date: NaiveDateTime,
    /// ID người tạo bản ghi
    pub created_by: Option<Uuid>,
    /// Ngày cập nhật vai trò gần nhất
    pub updated_date: NaiveDateTime,
    /// ID người cập nhật gần nhất
    pub updated_by: Option<Uuid>,
    /// Phiên bản vai trò phục vụ khóa lạc quan (Optimistic Locking)
    pub version: i32,
    /// Trạng thái xóa mềm (Soft delete)
    pub is_deleted: bool,
    /// Mã vai trò duy nhất (ví dụ: ROLE_ADMIN)
    pub code: String,
    /// Tên vai trò duy nhất (ví dụ: ADMIN, CUSTOMER)
    pub name: String,
    /// Mô tả chi tiết về quyền hạn và mục đích sử dụng vai trò này
    pub description: Option<String>,
    /// Danh sách các quyền hạn của vai trò (lưu dưới dạng chuỗi JSON)
    pub permissions: String,
}
