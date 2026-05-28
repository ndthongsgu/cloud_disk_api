use actix_web::{post, web, HttpResponse, Responder};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use crate::models::user::{User, UserStatus};
use crate::utils::password::{hash_password, verify_password};
use crate::utils::jwt::generate_token;

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

/// Phản hồi đăng nhập thành công chứa JWT Token
#[derive(Debug, Serialize)]
pub struct LoginResponse {
    /// Chuỗi JWT Token để lưu trữ và gửi kèm trong header Authorization
    pub token: String,
    /// ID tài khoản người dùng
    pub user_id: Uuid,
    /// Tên đăng nhập
    pub username: String,
}

/// API POST /auth/register - Đăng ký khách hàng mới sử dụng cơ chế Transaction để đảm bảo tính toàn vẹn
#[post("/auth/register")]
pub async fn register(
    db: web::Data<PgPool>,
    body: web::Json<RegisterRequest>,
) -> impl Responder {
    let request = body.into_inner();

    // Ràng buộc đơn giản đầu vào
    if request.username.trim().is_empty() || request.password.trim().is_empty() {
        return HttpResponse::BadRequest().body("Tên đăng nhập và mật khẩu không được để trống");
    }

    // Băm mật khẩu bảo mật
    let password_hash = match hash_password(&request.password) {
        Ok(h) => h,
        Err(_) => return HttpResponse::InternalServerError().body("Lỗi băm mật khẩu bảo mật"),
    };

    let user_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    // Khởi tạo một Database Transaction
    let mut tx = match db.begin().await {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().body("Lỗi khởi tạo Database Transaction"),
    };

    // 1. Tạo tài khoản trong bảng users
    let user_code = format!("USER-{}", &user_id.to_string()[..8].to_uppercase());
    let user_query = sqlx::query(
        "INSERT INTO users (id, created_date, created_by, updated_date, updated_by, version, is_deleted, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes) \
         VALUES ($1, $2, NULL, $3, NULL, 1, false, $4, $5, $6, $7, $8, $9, NULL, NULL, 10737418240, 0)"
    )
    .bind(user_id)
    .bind(now)
    .bind(now)
    .bind(&request.username)
    .bind(&password_hash)
    .bind("ACTIVE")
    .bind(user_code)
    .bind(&request.full_name)
    .bind(&request.email)
    .execute(&mut *tx)
    .await;

    if let Err(e) = user_query {
        return HttpResponse::Conflict().body(format!("Tên đăng nhập hoặc email đã tồn tại: {}", e));
    }

    // Commit Transaction
    if tx.commit().await.is_err() {
        return HttpResponse::InternalServerError().body("Không thể hoàn tất đăng ký tài khoản");
    }

    HttpResponse::Created().json("Đăng ký tài khoản khách hàng thành công!")
}

/// API POST /auth/login - Đăng nhập tài khoản và cấp JWT Token
#[post("/auth/login")]
pub async fn login(
    db: web::Data<PgPool>,
    body: web::Json<LoginRequest>,
) -> impl Responder {
    let request = body.into_inner();
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "my_secret_key".to_string());

    // Lấy thông tin tài khoản từ DB
    let user_res = sqlx::query_as::<_, User>(
        "SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes FROM users WHERE username = $1 AND is_deleted = false"
    )
    .bind(&request.username)
    .fetch_optional(db.get_ref())
    .await;

    let user = match user_res {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::Unauthorized().body("Tên đăng nhập hoặc mật khẩu không chính xác"),
        Err(_) => return HttpResponse::InternalServerError().body("Lỗi kết nối cơ sở dữ liệu"),
    };

    // Xác thực mật khẩu băm
    match verify_password(&request.password, &user.password_hash) {
        Ok(true) => {}
        _ => return HttpResponse::Unauthorized().body("Tên đăng nhập hoặc mật khẩu không chính xác"),
    }

    // Kiểm tra trạng thái tài khoản
    if user.status != UserStatus::Active {
        return HttpResponse::Forbidden().body("Tài khoản đang bị khóa hoặc bị cấm truy cập hệ thống");
    }

    // Tạo JWT Token có thời hạn 24 giờ (86400 giây)
    match generate_token(user.id, &user.username, &jwt_secret, 86400) {
        Ok(token) => HttpResponse::Ok().json(LoginResponse {
            token,
            user_id: user.id,
            username: user.username,
        }),
        Err(_) => HttpResponse::InternalServerError().body("Không thể khởi tạo JWT Token"),
    }
}
