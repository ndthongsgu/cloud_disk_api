use actix_web::{post, web, HttpResponse, Responder};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use crate::models::user::{User, UserStatus};
use crate::utils::password::{hash_password, verify_password};
use crate::utils::jwt::generate_token;
use crate::dtos::auth::{RegisterRequest, LoginRequest, LoginResponse};

/// API POST /auth/register - Đăng ký khách hàng mới sử dụng cơ chế Transaction để đảm bảo tính toàn vẹn
#[post("/api/auth/register")]
pub async fn register(
    db: web::Data<PgPool>,
    body: web::Json<RegisterRequest>,
) -> impl Responder {
    let request = body.into_inner();

    // Ràng buộc đơn giản đầu vào
    if request.username.trim().is_empty() || request.password.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Tên đăng nhập và mật khẩu không được để trống"
        }));
    }

    // Băm mật khẩu bảo mật
    let password_hash = match hash_password(&request.password) {
        Ok(h) => h,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi băm mật khẩu bảo mật"
        })),
    };

    let user_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    // Khởi tạo một Database Transaction
    let mut tx = match db.begin().await {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi hệ thống khi khởi tạo phiên làm việc dữ liệu"
        })),
    };

    // 1. Tạo tài khoản trong bảng users
    let user_code = format!("USER-{}", &user_id.to_string()[..8].to_uppercase());
    let user_query = sqlx::query(
        "INSERT INTO users (id, created_date, created_by, updated_date, updated_by, version, is_deleted, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes) \
         VALUES ($1, $2, NULL, $3, NULL, 1, false, $4, $5, $6, $7, $8, $9, NULL, NULL, 0, 0)"
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

    if let Err(_) = user_query {
        return HttpResponse::Conflict().json(serde_json::json!({
            "message": "Tên đăng nhập hoặc địa chỉ email đã tồn tại"
        }));
    }

    // Commit Transaction
    if tx.commit().await.is_err() {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Không thể hoàn tất quá trình đăng ký tài khoản"
        }));
    }

    // Tạo JWT Token có thời hạn 24 giờ (86400 giây) và Refresh Token 7 ngày (604800 giây)
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "my_secret_key".to_string());
    let jwt_refresh_secret = std::env::var("JWT_REFRESH_SECRET").unwrap_or_else(|_| "my_refresh_secret_key".to_string());

    let access_token = match generate_token(user_id, &request.username, &jwt_secret, 86400) {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Không thể khởi tạo mã xác thực JWT Access Token"
        })),
    };

    let refresh_token = match generate_token(user_id, &request.username, &jwt_refresh_secret, 604800) {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Không thể khởi tạo mã xác thực JWT Refresh Token"
        })),
    };

    HttpResponse::Created().json(LoginResponse {
        token: access_token,
        refresh_token,
        user_id,
        email: request.email,
        full_name: request.full_name,
        username: request.username,
        is_admin: false,
    })
}

/// API POST /auth/login - Đăng nhập tài khoản và cấp JWT Token
#[post("/api/auth/login")]
pub async fn login(
    db: web::Data<PgPool>,
    body: web::Json<LoginRequest>,
) -> impl Responder {
    let request = body.into_inner();
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "my_secret_key".to_string());

    // Lấy thông tin tài khoản từ DB
    let user_res = sqlx::query_as::<_, User>(
        "SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, is_admin, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes FROM users WHERE username = $1 AND is_deleted = false"
    )
    .bind(&request.username)
    .fetch_optional(db.get_ref())
    .await;

    let user = match user_res {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::Unauthorized().json(serde_json::json!({
            "message": "Tên đăng nhập hoặc mật khẩu không chính xác"
        })),
        Err(e) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": format!("Không thể kết nối đến cơ sở dữ liệu hệ thống : {}", e)
        })),
    };

    // Xác thực mật khẩu băm
    match verify_password(&request.password, &user.password_hash) {
        Ok(true) => {}
        _ => return HttpResponse::Unauthorized().json(serde_json::json!({
            "message": "Tên đăng nhập hoặc mật khẩu không chính xác"
        })),
    }

    // Kiểm tra trạng thái tài khoản
    if user.status != UserStatus::Active {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Tài khoản đang bị khóa hoặc bị cấm truy cập hệ thống"
        }));
    }

    // Tạo JWT Token có thời hạn 24 giờ (86400 giây) và Refresh Token 7 ngày (604800 giây)
    let jwt_refresh_secret = std::env::var("JWT_REFRESH_SECRET").unwrap_or_else(|_| "my_refresh_secret_key".to_string());
    
    let access_token = match generate_token(user.id, &user.username, &jwt_secret, 86400) {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Không thể khởi tạo mã xác thực JWT Access Token"
        })),
    };

    let refresh_token = match generate_token(user.id, &user.username, &jwt_refresh_secret, 604800) {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Không thể khởi tạo mã xác thực JWT Refresh Token"
        })),
    };

    HttpResponse::Ok().json(LoginResponse {
        token: access_token,
        refresh_token,
        user_id: user.id,
        username: user.username,
        full_name: user.full_name,
        email: user.email,
        is_admin: user.is_admin,
    })
}

/// API POST /auth/refresh - Cấp Access Token & Refresh Token mới bằng Refresh Token hợp lệ
#[post("/api/auth/refresh")]
pub async fn refresh(
    db: web::Data<PgPool>,
    body: web::Json<crate::dtos::auth::RefreshTokenRequest>,
) -> impl Responder {
    let request = body.into_inner();
    let jwt_secret = std::env::var("JWT_SECRET").unwrap_or_else(|_| "my_secret_key".to_string());
    let jwt_refresh_secret = std::env::var("JWT_REFRESH_SECRET").unwrap_or_else(|_| "my_refresh_secret_key".to_string());

    // 1. Xác thực và giải mã Refresh Token
    let claims = match crate::utils::jwt::verify_token(&request.refresh_token, &jwt_refresh_secret) {
        Ok(c) => c,
        Err(_) => return HttpResponse::Unauthorized().json(serde_json::json!({
            "message": "Mã refresh token không hợp lệ hoặc đã hết hạn sử dụng"
        })),
    };

    // 2. Lấy thông tin người dùng từ DB để đảm bảo tài khoản vẫn hoạt động bình thường
    let user_res = sqlx::query_as::<_, User>(
        "SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes FROM users WHERE id = $1 AND is_deleted = false"
    )
    .bind(claims.sub)
    .fetch_optional(db.get_ref())
    .await;

    let user = match user_res {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::Unauthorized().json(serde_json::json!({
            "message": "Tài khoản người dùng không tồn tại hoặc đã bị xóa khỏi hệ thống"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Không thể kết nối đến cơ sở dữ liệu hệ thống"
        })),
    };

    // 3. Kiểm tra trạng thái hoạt động của tài khoản
    if user.status != UserStatus::Active {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Tài khoản đang bị khóa hoặc bị cấm truy cập hệ thống"
        }));
    }

    // 4. Sinh cặp Token mới (Rotating Refresh Token)
    let new_access_token = match crate::utils::jwt::generate_token(user.id, &user.username, &jwt_secret, 86400) {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Không thể tạo mới mã Access Token"
        })),
    };

    let new_refresh_token = match crate::utils::jwt::generate_token(user.id, &user.username, &jwt_refresh_secret, 604800) {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Không thể tạo mới mã Refresh Token"
        })),
    };

    HttpResponse::Ok().json(LoginResponse {
        token: new_access_token,
        refresh_token: new_refresh_token,
        user_id: user.id,
        username: user.username,
        full_name: user.full_name,
        email: user.email,
        is_admin: user.is_admin,
    })
}
