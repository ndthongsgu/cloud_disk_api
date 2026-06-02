use chrono::NaiveDateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Trạng thái hoạt động của tài khoản người dùng
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, sqlx::Type)]
#[sqlx(type_name = "varchar", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum UserStatus {
    /// Đang hoạt động bình thường
    Active,
    /// Đang tạm khóa tài khoản
    Locked,
    /// Bị cấm khỏi hệ thống
    Banned,
}

/// Mô hình quản lý tài khoản người dùng và thông tin xác thực
#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct User {
    /// ID duy nhất của người dùng
    pub id: Uuid,
    /// Ngày tạo tài khoản
    pub created_date: NaiveDateTime,
    /// ID người tạo bản ghi
    pub created_by: Option<Uuid>,
    /// Ngày cập nhật tài khoản gần nhất
    pub updated_date: NaiveDateTime,
    /// ID người cập nhật gần nhất
    pub updated_by: Option<Uuid>,
    /// Phiên bản bản ghi phục vụ khóa lạc quan (Optimistic Locking)
    pub version: i32,
    /// Trạng thái xóa mềm (Soft delete)
    pub is_deleted: bool,
    /// Tài khoản admin được khởi tạo từ đầu
    pub is_admin: bool,
    /// Tên đăng nhập duy nhất
    pub username: String,
    /// Mã băm mật khẩu bảo mật (bcrypt, argon2...)
    pub password_hash: String,
    /// Trạng thái tài khoản người dùng
    pub status: UserStatus,
    /// Mã định danh người dùng duy nhất (ví dụ: USER0001)
    pub code: String,
    /// Họ và tên đầy đủ của người dùng
    pub full_name: String,
    /// Địa chỉ email đăng ký duy nhất của người dùng
    pub email: String,
    /// Số điện thoại liên lạc của người dùng
    pub phone: Option<String>,
    /// Tên phòng ban người dùng đang làm việc (nếu là nhân viên)
    pub department_name: Option<String>,
    /// Hạn mức dung lượng lưu trữ cho phép (byte, để NULL nếu không giới hạn)
    pub storage_quota_bytes: Option<i64>,
    /// Tổng dung lượng tệp tin khách hàng đã tải lên thực tế (byte)
    pub used_storage_bytes: i64,
}

