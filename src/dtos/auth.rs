use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Yêu cầu đăng ký tài khoản mới từ phía Client
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    /// Tên đăng nhập
    pub username: String,
    /// Mật khẩu trần
    pub password: String,
    /// Họ và tên đầy đủ khách hàng
    pub full_name: String,
    /// Email đăng ký
    pub email: String,
}

/// Yêu cầu đăng nhập tài khoản
#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    /// Tên đăng nhập
    pub username: String,
    /// Mật khẩu trần
    pub password: String,
}

/// Phản hồi đăng nhập thành công chứa JWT Token & Refresh Token
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    /// Chuỗi Access JWT Token để lưu trữ và gửi kèm trong header Authorization
    pub token: String,
    /// Chuỗi Refresh Token để cấp lại Access Token mới
    pub refresh_token: String,
    /// ID tài khoản người dùng
    pub user_id: Uuid,
    /// Tên đăng nhập
    pub username: String,
    /// email
    pub email: String,
    /// Họ và tên đầy đủ khách hàng
    pub full_name: String,
    /// Cờ admin trên backend
    pub is_admin: bool,
}

/// Yêu cầu cấp lại Access Token mới từ phía Client
#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    /// Chuỗi Refresh Token
    pub refresh_token: String,
}
