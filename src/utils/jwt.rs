use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Cấu trúc Claims được lưu trữ bên trong JWT Payload
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    /// ID duy nhất của tài khoản người dùng sở hữu token (subject)
    pub sub: Uuid,
    /// Tên đăng nhập của tài khoản người dùng
    pub username: String,
    /// Thời gian hết hạn của Token dạng UNIX timestamp
    pub exp: usize,
}

/// Tạo chuỗi JWT Token ký bằng thuật toán HS256 bảo mật
pub fn generate_token(
    user_id: Uuid,
    username: &str,
    secret: &str,
    expires_in_seconds: i64,
) -> Result<String, jsonwebtoken::errors::Error> {
    let expiration = Utc::now()
        .checked_add_signed(Duration::seconds(expires_in_seconds))
        .expect("Mốc thời gian hết hạn không hợp lệ")
        .timestamp();

    let claims = Claims {
        sub: user_id,
        username: username.to_string(),
        exp: expiration as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
}

/// Xác thực chuỗi JWT Token và trích xuất Claims chứa thông tin tài khoản
pub fn verify_token(token: &str, secret: &str) -> Result<Claims, jsonwebtoken::errors::Error> {
    let mut validation = Validation::default();
    validation.validate_exp = true; // Bắt buộc kiểm tra mốc thời gian hết hạn

    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )?;

    Ok(token_data.claims)
}
