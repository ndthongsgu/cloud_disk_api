use actix_web::{get, post, put, web, HttpResponse, Responder};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

use crate::middlewares::auth::UserClaims;
use crate::models::user::User;
use crate::dtos::user::{
    UserSearchRequest, CreateUserRequest, UpdateUserRequest, UserResponse,
    UserSelectRequest, UserSelectResponse,
};
use crate::utils::permission::{verify_permission, check_is_admin};
use crate::common::constants::permission::{SystemModule, SystemAction};
use crate::utils::password::hash_password;

/// Chuyển đổi struct model User và danh sách vai trò sang UserResponse DTO để trả về Client
fn to_response(user: User, roles: Vec<Uuid>) -> UserResponse {
    let status_str = match user.status {
        crate::models::user::UserStatus::Active => "ACTIVE".to_string(),
        crate::models::user::UserStatus::Locked => "LOCKED".to_string(),
        crate::models::user::UserStatus::Banned => "BANNED".to_string(),
    };
    UserResponse {
        id: user.id,
        created_date: user.created_date,
        created_by: user.created_by,
        updated_date: user.updated_date,
        updated_by: user.updated_by,
        version: user.version,
        is_admin: user.is_admin,
        username: user.username,
        status: status_str,
        code: user.code,
        full_name: user.full_name,
        email: user.email,
        phone: user.phone,
        department_name: user.department_name,
        storage_quota_bytes: user.storage_quota_bytes,
        used_storage_bytes: user.used_storage_bytes,
        roles,
    }
}

/// Helper lấy danh sách ID vai trò gán cho người dùng
async fn get_user_role_ids(db: &PgPool, user_id: Uuid) -> Result<Vec<Uuid>, sqlx::Error> {
    let rows = sqlx::query!(
        r#"
        SELECT role_id 
        FROM user_roles 
        WHERE user_id = $1 AND is_deleted = false
        "#,
        user_id
    )
    .fetch_all(db)
    .await?;
    
    Ok(rows.into_iter().map(|r| r.role_id).collect())
}

/// API POST /users/search - Danh sách người dùng có phân trang và bộ lọc (Filter)
#[post("/api/users/search")]
pub async fn search_users(
    db: web::Data<PgPool>,
    req: web::Json<UserSearchRequest>,
    claims: UserClaims,
) -> impl Responder {
    // 1. Kiểm tra quyền hạn vai trò (RBAC) - Yêu cầu quyền "view" trong module "users"
    match verify_permission(db.get_ref(), claims.user_id, SystemModule::Users, SystemAction::View).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền xem danh sách người dùng"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    let is_admin = check_is_admin(db.get_ref(), claims.user_id).await.unwrap_or(false);
    let body = req.into_inner();
    let limit = body.size.unwrap_or(10).clamp(1, 100);
    let offset = body.page.unwrap_or(1).saturating_sub(1) * limit;

    let username_pat = body.username.as_ref().map(|x| format!("%{}%", x.trim()));
    let full_name_pat = body.full_name.as_ref().map(|x| format!("%{}%", x.trim()));
    let email_pat = body.email.as_ref().map(|x| format!("%{}%", x.trim()));
    let phone_pat = body.phone.as_ref().map(|x| format!("%{}%", x.trim()));
    let dept_pat = body.department_name.as_ref().map(|x| format!("%{}%", x.trim()));
    let status_val = body.status.as_ref().map(|x| x.trim().to_uppercase());

    // 2. Tính tổng số bản ghi (Áp dụng Tenant Isolation)
    // - Admin: Xem toàn bộ users
    // - Người dùng thường (Tenant): Chỉ xem users do chính mình tạo ra
    let total = if is_admin {
        match sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM users
            WHERE is_deleted = false
              AND ($1::varchar IS NULL OR username ILIKE $1)
              AND ($2::varchar IS NULL OR full_name ILIKE $2)
              AND ($3::varchar IS NULL OR email ILIKE $3)
              AND ($4::varchar IS NULL OR phone ILIKE $4)
              AND ($5::varchar IS NULL OR department_name ILIKE $5)
              AND ($6::varchar IS NULL OR status = $6)
            "#,
            username_pat,
            full_name_pat,
            email_pat,
            phone_pat,
            dept_pat,
            status_val
        )
        .fetch_one(db.get_ref())
        .await {
            Ok(r) => r.count,
            Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "Lỗi hệ thống khi tính tổng số lượng người dùng"
            })),
        }
    } else {
        match sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!"
            FROM users
            WHERE is_deleted = false
              AND created_by = $1
              AND ($2::varchar IS NULL OR username ILIKE $2)
              AND ($3::varchar IS NULL OR full_name ILIKE $3)
              AND ($4::varchar IS NULL OR email ILIKE $4)
              AND ($5::varchar IS NULL OR phone ILIKE $5)
              AND ($6::varchar IS NULL OR department_name ILIKE $6)
              AND ($7::varchar IS NULL OR status = $7)
            "#,
            claims.user_id,
            username_pat,
            full_name_pat,
            email_pat,
            phone_pat,
            dept_pat,
            status_val
        )
        .fetch_one(db.get_ref())
        .await {
            Ok(r) => r.count,
            Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "Lỗi hệ thống khi tính tổng số lượng người dùng"
            })),
        }
    };

    // 3. Lấy danh sách người dùng phân trang
    let users_res = if is_admin {
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, is_admin, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes
            FROM users
            WHERE is_deleted = false
              AND ($1::varchar IS NULL OR username ILIKE $1)
              AND ($2::varchar IS NULL OR full_name ILIKE $2)
              AND ($3::varchar IS NULL OR email ILIKE $3)
              AND ($4::varchar IS NULL OR phone ILIKE $4)
              AND ($5::varchar IS NULL OR department_name ILIKE $5)
              AND ($6::varchar IS NULL OR status = $6)
            ORDER BY created_date DESC
            LIMIT $7 OFFSET $8
            "#
        )
        .bind(username_pat)
        .bind(full_name_pat)
        .bind(email_pat)
        .bind(phone_pat)
        .bind(dept_pat)
        .bind(status_val)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(db.get_ref())
        .await
    } else {
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, is_admin, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes
            FROM users
            WHERE is_deleted = false
              AND created_by = $1
              AND ($2::varchar IS NULL OR username ILIKE $2)
              AND ($3::varchar IS NULL OR full_name ILIKE $3)
              AND ($4::varchar IS NULL OR email ILIKE $4)
              AND ($5::varchar IS NULL OR phone ILIKE $5)
              AND ($6::varchar IS NULL OR department_name ILIKE $6)
              AND ($7::varchar IS NULL OR status = $7)
            ORDER BY created_date DESC
            LIMIT $8 OFFSET $9
            "#
        )
        .bind(claims.user_id)
        .bind(username_pat)
        .bind(full_name_pat)
        .bind(email_pat)
        .bind(phone_pat)
        .bind(dept_pat)
        .bind(status_val)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(db.get_ref())
        .await
    };

    match users_res {
        Ok(items) => {
            let mut responses = Vec::new();
            for u in items {
                let role_ids = get_user_role_ids(db.get_ref(), u.id).await.unwrap_or_default();
                responses.push(to_response(u, role_ids));
            }
            HttpResponse::Ok().json(serde_json::json!({
                "items": responses,
                "total": total,
                "page": body.page.unwrap_or(1),
                "size": limit,
            }))
        }
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi truy vấn danh sách người dùng"
        })),
    }
}

/// API GET /users/{id} - Xem chi tiết người dùng
#[get("/api/users/{id}")]
pub async fn get_user_detail(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
    claims: UserClaims,
) -> impl Responder {
    let target_user_id = path.into_inner();

    // 1. Kiểm tra quyền hạn vai trò (RBAC) - Yêu cầu quyền "view" trong module "users"
    match verify_permission(db.get_ref(), claims.user_id, SystemModule::Users, SystemAction::View).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền xem chi tiết người dùng"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    let is_admin = check_is_admin(db.get_ref(), claims.user_id).await.unwrap_or(false);

    // 2. Lấy thông tin user
    let user_res = sqlx::query_as::<_, User>(
        r#"
        SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, is_admin, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes
        FROM users
        WHERE id = $1 AND is_deleted = false
        "#
    )
    .bind(target_user_id)
    .fetch_optional(db.get_ref())
    .await;

    match user_res {
        Ok(Some(u)) => {
            // 3. Kiểm tra Tenant Isolation:
            // Non-admin chỉ được xem chi tiết chính họ hoặc user do chính họ tạo ra
            if !is_admin && u.created_by != Some(claims.user_id) && u.id != claims.user_id {
                return HttpResponse::Forbidden().json(serde_json::json!({
                    "message": "Bạn không có quyền truy cập thông tin tài khoản của người dùng khác"
                }));
            }
            
            let role_ids = get_user_role_ids(db.get_ref(), u.id).await.unwrap_or_default();
            HttpResponse::Ok().json(to_response(u, role_ids))
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "message": "Không tìm thấy thông tin tài khoản người dùng"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi truy vấn thông tin người dùng"
        })),
    }
}

/// API POST /users - Thêm mới người dùng & gán vai trò
#[post("/api/users")]
pub async fn create_user(
    db: web::Data<PgPool>,
    req: web::Json<CreateUserRequest>,
    claims: UserClaims,
) -> impl Responder {
    // 1. Kiểm tra quyền RBAC - Yêu cầu quyền "add" trong module "users"
    match verify_permission(db.get_ref(), claims.user_id, SystemModule::Users, SystemAction::Add).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền thêm người dùng mới"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    let body = req.into_inner();
    let username = body.username.trim().to_string();
    let password = body.password.trim().to_string();
    let full_name = body.full_name.trim().to_string();
    let email = body.email.trim().to_string();

    if username.is_empty() || password.is_empty() || full_name.is_empty() || email.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Tên đăng nhập, mật khẩu, họ tên và địa chỉ email không được để trống"
        }));
    }

    // 2. Kiểm tra trùng username hoặc email
    let exists_res = sqlx::query!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM users
            WHERE is_deleted = false AND (username = $1 OR email = $2)
        ) as "exists!"
        "#,
        username,
        email
    )
    .fetch_one(db.get_ref())
    .await;

    match exists_res {
        Ok(r) if r.exists => return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Tên đăng nhập hoặc địa chỉ email đã tồn tại trong hệ thống"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra trùng lặp"
        })),
        _ => {}
    }

    // 3. Mã hóa mật khẩu
    let password_hash = match hash_password(&password) {
        Ok(h) => h,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi mã hóa bảo mật mật khẩu"
        })),
    };

    let new_user_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();
    let user_code = format!("USER-{}", &new_user_id.to_string()[..8].to_uppercase());

    // 4. Bắt đầu Database Transaction để tạo User và gán Roles
    let mut tx = match db.begin().await {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi khởi tạo giao dịch (transaction)"
        })),
    };

    let user_insert = sqlx::query(
        r#"
        INSERT INTO users (id, created_date, created_by, updated_date, updated_by, version, is_deleted, is_admin, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes)
        VALUES ($1, $2, $3, $4, $5, 1, false, false, $6, $7, $8, $9, $10, $11, $12, $13, $14, 0)
        "#
    )
    .bind(new_user_id)
    .bind(now)
    .bind(claims.user_id)
    .bind(now)
    .bind(claims.user_id)
    .bind(&username)
    .bind(password_hash)
    .bind("ACTIVE")
    .bind(user_code)
    .bind(full_name)
    .bind(email)
    .bind(&body.phone)
    .bind(&body.department_name)
    .bind(body.storage_quota_bytes.unwrap_or(10737418240)) // mặc định 10GB
    .execute(&mut *tx)
    .await;

    if let Err(_) = user_insert {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi thêm tài khoản người dùng"
        }));
    }

    // 5. Gán vai trò
    for role_id in &body.roles {
        let role_exists = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM roles WHERE id = $1 AND is_deleted = false) as \"exists!\"",
            role_id
        )
        .fetch_one(&mut *tx)
        .await;

        match role_exists {
            Ok(r) if r.exists => {
                let ur_id = Uuid::new_v4();
                let ur_insert = sqlx::query(
                    r#"
                    INSERT INTO user_roles (id, created_date, created_by, updated_date, updated_by, version, is_deleted, user_id, role_id)
                    VALUES ($1, $2, $3, $4, $5, 1, false, $6, $7)
                    "#
                )
                .bind(ur_id)
                .bind(now)
                .bind(claims.user_id)
                .bind(now)
                .bind(claims.user_id)
                .bind(new_user_id)
                .bind(role_id)
                .execute(&mut *tx)
                .await;

                if let Err(_) = ur_insert {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "message": "Lỗi liên kết vai trò"
                    }));
                }
            }
            _ => {
                return HttpResponse::BadRequest().json(serde_json::json!({
                    "message": format!("Vai trò ID {} không tồn tại hoặc đã bị xóa", role_id)
                }));
            }
        }
    }

    if tx.commit().await.is_err() {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Không thể hoàn tất đăng ký tài khoản"
        }));
    }

    // Lấy thông tin user vừa tạo để trả về
    let u_created = sqlx::query_as::<_, User>(
        r#"
        SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, is_admin, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes
        FROM users
        WHERE id = $1
        "#
    )
    .bind(new_user_id)
    .fetch_one(db.get_ref())
    .await;

    match u_created {
        Ok(u) => HttpResponse::Created().json(to_response(u, body.roles)),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi phản hồi thông tin tài khoản"
        })),
    }
}

/// API PUT /users/{id} - Sửa đổi thông tin người dùng & vai trò
#[put("/api/users/{id}")]
pub async fn update_user(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: web::Json<UpdateUserRequest>,
    claims: UserClaims,
) -> impl Responder {
    let target_user_id = path.into_inner();

    // 1. Kiểm tra quyền RBAC - Yêu cầu quyền "edit" trong module "users"
    match verify_permission(db.get_ref(), claims.user_id, SystemModule::Users, SystemAction::Edit).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền chỉnh sửa thông tin người dùng"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    let is_admin = check_is_admin(db.get_ref(), claims.user_id).await.unwrap_or(false);

    // 2. Lấy thông tin user hiện tại
    let current_user = match sqlx::query_as::<_, User>(
        r#"
        SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, is_admin, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes
        FROM users
        WHERE id = $1 AND is_deleted = false
        "#
    )
    .bind(target_user_id)
    .fetch_optional(db.get_ref())
    .await {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({
            "message": "Không tìm thấy người dùng cần sửa đổi"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi truy vấn tài khoản"
        })),
    };

    // 3. Kiểm tra Tenant Isolation
    // Non-admin chỉ được sửa thông tin chính mình hoặc user do mình tạo ra
    if !is_admin && current_user.created_by != Some(claims.user_id) && current_user.id != claims.user_id {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền chỉnh sửa tài khoản người dùng khác"
        }));
    }

    let body = req.into_inner();
    let full_name = body.full_name.trim().to_string();
    let email = body.email.trim().to_string();

    if full_name.is_empty() || email.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Họ tên và địa chỉ email không được để trống"
        }));
    }

    // 4. Kiểm tra trùng email với tài khoản khác
    let email_exists = sqlx::query!(
        r#"
        SELECT EXISTS(
            SELECT 1 FROM users
            WHERE is_deleted = false AND email = $1 AND id <> $2
        ) as "exists!"
        "#,
        email,
        target_user_id
    )
    .fetch_one(db.get_ref())
    .await;

    match email_exists {
        Ok(r) if r.exists => return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Địa chỉ email đã được sử dụng bởi một tài khoản khác"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra trùng lặp email"
        })),
        _ => {}
    }

    let now = Utc::now().naive_utc();

    // 5. Khởi tạo Database Transaction để cập nhật User và Role liên kết
    let mut tx = match db.begin().await {
        Ok(t) => t,
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi khởi tạo phiên làm việc dữ liệu"
        })),
    };

    // Cập nhật bảng users (sử dụng Optimistic locking qua version)
    let user_update = sqlx::query_as::<_, User>(
        r#"
        UPDATE users
        SET full_name = $1,
            email = $2,
            phone = $3,
            department_name = $4,
            storage_quota_bytes = $5,
            updated_date = $6,
            updated_by = $7,
            version = version + 1
        WHERE id = $8 AND version = $9 AND is_deleted = false
        RETURNING id, created_date, created_by, updated_date, updated_by, version, is_deleted, is_admin, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes
        "#
    )
    .bind(full_name)
    .bind(email)
    .bind(&body.phone)
    .bind(&body.department_name)
    .bind(body.storage_quota_bytes)
    .bind(now)
    .bind(claims.user_id)
    .bind(target_user_id)
    .bind(current_user.version)
    .fetch_optional(&mut *tx)
    .await;

    let updated_user = match user_update {
        Ok(Some(u)) => u,
        Ok(None) => {
            return HttpResponse::Conflict().json(serde_json::json!({
                "message": "Thông tin tài khoản đã được cập nhật bởi một phiên làm việc khác, vui lòng tải lại trang"
            }));
        }
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi cập nhật thông tin người dùng"
        })),
    };

    // Xóa tất cả các vai trò cũ trong bảng user_roles (mối quan hệ n-n)
    let delete_ur = sqlx::query("DELETE FROM user_roles WHERE user_id = $1")
        .bind(target_user_id)
        .execute(&mut *tx)
        .await;

    if let Err(_) = delete_ur {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi xóa cấu hình vai trò cũ"
        }));
    }

    // Thêm các vai trò mới
    for role_id in &body.roles {
        let role_exists = sqlx::query!(
            "SELECT EXISTS(SELECT 1 FROM roles WHERE id = $1 AND is_deleted = false) as \"exists!\"",
            role_id
        )
        .fetch_one(&mut *tx)
        .await;

        match role_exists {
            Ok(r) if r.exists => {
                let ur_id = Uuid::new_v4();
                let ur_insert = sqlx::query(
                    r#"
                    INSERT INTO user_roles (id, created_date, created_by, updated_date, updated_by, version, is_deleted, user_id, role_id)
                    VALUES ($1, $2, $3, $4, $5, 1, false, $6, $7)
                    "#
                )
                .bind(ur_id)
                .bind(now)
                .bind(claims.user_id)
                .bind(now)
                .bind(claims.user_id)
                .bind(target_user_id)
                .bind(role_id)
                .execute(&mut *tx)
                .await;

                if let Err(_) = ur_insert {
                    return HttpResponse::InternalServerError().json(serde_json::json!({
                        "message": "Lỗi gán vai trò mới"
                    }));
                }
            }
            _ => return HttpResponse::BadRequest().json(serde_json::json!({
                "message": format!("Vai trò ID {} không tồn tại", role_id)
            })),
        }
    }

    if tx.commit().await.is_err() {
        return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Không thể hoàn tất cập nhật tài khoản"
        }));
    }

    HttpResponse::Ok().json(to_response(updated_user, body.roles))
}

/// API POST /users/{id}/deactivate - Ngưng hoạt động người dùng (Khóa tài khoản)
#[post("/api/users/{id}/deactivate")]
pub async fn deactivate_user(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
    claims: UserClaims,
) -> impl Responder {
    let target_user_id = path.into_inner();

    // 1. Kiểm tra quyền RBAC - Yêu cầu quyền "deactive" trong module "users"
    match verify_permission(db.get_ref(), claims.user_id, SystemModule::Users, SystemAction::DeActive).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền vô hiệu hóa hoạt động của tài khoản người dùng"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    let is_admin = check_is_admin(db.get_ref(), claims.user_id).await.unwrap_or(false);

    // 2. Lấy thông tin user
    let user_res = sqlx::query_as::<_, User>(
        r#"
        SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, is_admin, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes
        FROM users
        WHERE id = $1 AND is_deleted = false
        "#
    )
    .bind(target_user_id)
    .fetch_optional(db.get_ref())
    .await;

    let user = match user_res {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({
            "message": "Không tìm thấy người dùng trong hệ thống"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi cơ sở dữ liệu"
        })),
    };

    // 3. Ràng buộc an toàn: Không được tự khóa chính mình
    if user.id == claims.user_id {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Không thể tự vô hiệu hóa tài khoản của chính mình"
        }));
    }

    // 4. Kiểm tra Tenant Isolation
    // Non-admin chỉ được khóa tài khoản do chính mình tạo ra
    if !is_admin && user.created_by != Some(claims.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền vô hiệu hóa tài khoản của người dùng khác"
        }));
    }

    // 5. Khóa tài khoản (status = LOCKED)
    let now = Utc::now().naive_utc();
    let update_res = sqlx::query(
        r#"
        UPDATE users
        SET status = $1,
            updated_date = $2,
            updated_by = $3,
            version = version + 1
        WHERE id = $4 AND is_deleted = false
        "#
    )
    .bind("LOCKED")
    .bind(now)
    .bind(claims.user_id)
    .bind(target_user_id)
    .execute(db.get_ref())
    .await;

    match update_res {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Đã khóa tài khoản người dùng thành công!"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi cập nhật trạng thái hoạt động"
        })),
    }
}

/// API POST /users/{id}/activate - Hoạt động lại người dùng (Mở khóa tài khoản)
#[post("/api/users/{id}/activate")]
pub async fn activate_user(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
    claims: UserClaims,
) -> impl Responder {
    let target_user_id = path.into_inner();

    // 1. Kiểm tra quyền RBAC - Yêu cầu quyền "active" trong module "users"
    match verify_permission(db.get_ref(), claims.user_id, SystemModule::Users, SystemAction::Active).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền kích hoạt lại tài khoản người dùng"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    let is_admin = check_is_admin(db.get_ref(), claims.user_id).await.unwrap_or(false);

    // 2. Lấy thông tin user
    let user_res = sqlx::query_as::<_, User>(
        r#"
        SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, is_admin, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes
        FROM users
        WHERE id = $1 AND is_deleted = false
        "#
    )
    .bind(target_user_id)
    .fetch_optional(db.get_ref())
    .await;

    let user = match user_res {
        Ok(Some(u)) => u,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({
            "message": "Không tìm thấy người dùng trong hệ thống"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi cơ sở dữ liệu"
        })),
    };

    // 3. Kiểm tra Tenant Isolation
    // Non-admin chỉ được kích hoạt tài khoản do chính mình tạo ra
    if !is_admin && user.created_by != Some(claims.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền kích hoạt tài khoản của người dùng khác"
        }));
    }

    // 4. Kích hoạt tài khoản (status = ACTIVE)
    let now = Utc::now().naive_utc();
    let update_res = sqlx::query(
        r#"
        UPDATE users
        SET status = $1,
            updated_date = $2,
            updated_by = $3,
            version = version + 1
        WHERE id = $4 AND is_deleted = false
        "#
    )
    .bind("ACTIVE")
    .bind(now)
    .bind(claims.user_id)
    .bind(target_user_id)
    .execute(db.get_ref())
    .await;

    match update_res {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Kích hoạt hoạt động tài khoản thành công!"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kích hoạt tài khoản"
        })),
    }
}

/// API POST /users/select - Lightweight select box dropdown list có bộ lọc (Filter)
#[post("/api/users/select")]
pub async fn select_users(
    db: web::Data<PgPool>,
    req: web::Json<UserSelectRequest>,
    claims: UserClaims,
) -> impl Responder {
    // 1. Kiểm tra quyền RBAC - Yêu cầu quyền "view" trong module "users"
    match verify_permission(db.get_ref(), claims.user_id, SystemModule::Users, SystemAction::View).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền xem thông tin người dùng"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    let is_admin = check_is_admin(db.get_ref(), claims.user_id).await.unwrap_or(false);
    let body = req.into_inner();
    let limit = body.limit.unwrap_or(20).clamp(1, 100);
    let search_pat = body.q.as_ref().map(|x| format!("%{}%", x.trim()));

    // 2. Truy vấn danh sách gọn nhẹ (Áp dụng Tenant Isolation)
    // - Admin: Tìm kiếm trên toàn bộ users
    // - Non-admin: Chỉ tìm kiếm trên chính mình hoặc users do chính mình tạo ra
    let users_res = if is_admin {
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, is_admin, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes
            FROM users
            WHERE is_deleted = false
              AND ($1::varchar IS NULL OR username ILIKE $1 OR full_name ILIKE $1 OR email ILIKE $1)
            ORDER BY username ASC
            LIMIT $2
            "#
        )
        .bind(search_pat)
        .bind(limit as i64)
        .fetch_all(db.get_ref())
        .await
    } else {
        sqlx::query_as::<_, User>(
            r#"
            SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, is_admin, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes
            FROM users
            WHERE is_deleted = false
              AND (created_by = $1 OR id = $1)
              AND ($2::varchar IS NULL OR username ILIKE $2 OR full_name ILIKE $2 OR email ILIKE $2)
            ORDER BY username ASC
            LIMIT $3
            "#
        )
        .bind(claims.user_id)
        .bind(search_pat)
        .bind(limit as i64)
        .fetch_all(db.get_ref())
        .await
    };

    match users_res {
        Ok(items) => {
            let responses: Vec<UserSelectResponse> = items.into_iter().map(|u| {
                UserSelectResponse {
                    id: u.id,
                    username: u.username,
                    full_name: u.full_name,
                    code: u.code,
                }
            }).collect();
            HttpResponse::Ok().json(responses)
        }
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi tải dữ liệu select box"
        })),
    }
}
