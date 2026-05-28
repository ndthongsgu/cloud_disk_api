use std::future::{ready, Ready};
use actix_web::{dev::Payload, error::ErrorUnauthorized, FromRequest, HttpRequest, Error};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use crate::utils::jwt::verify_token;

/// Thông tin tài khoản người dùng trích xuất từ JWT hợp lệ
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct UserClaims {
    /// ID duy nhất của tài khoản người dùng
    pub user_id: Uuid,
    /// Tên đăng nhập của tài khoản
    pub username: String,
}

impl FromRequest for UserClaims {
    type Error = Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, _payload: &mut Payload) -> Self::Future {
        // Trích xuất JWT Secret từ biến môi trường (.env)
        let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "my_secret_key".to_string());

        // Kiểm tra xem header "Authorization" có tồn tại không
        if let Some(auth_header) = req.headers().get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                // Kiểm tra định dạng "Bearer <token>"
                if auth_str.starts_with("Bearer ") {
                    let token = auth_str.trim_start_matches("Bearer ");
                    // Xác thực chữ ký token và thời gian hết hạn
                    match verify_token(token, &jwt_secret) {
                        Ok(claims) => {
                            return ready(Ok(UserClaims {
                                user_id: claims.sub,
                                username: claims.username,
                            }));
                        }
                        Err(_) => {
                            return ready(Err(ErrorUnauthorized("Token xác thực không hợp lệ hoặc đã hết hạn")));
                        }
                    }
                }
            }
        }

        ready(Err(ErrorUnauthorized("Yêu cầu bị từ chối: Thiếu header Authorization chứa Bearer Token")))
    }
}
