use actix_web::{get, post, put, delete, web, HttpResponse, Responder};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

use crate::middlewares::auth::UserClaims;
use crate::models::role::Role;
use crate::dtos::role::{RoleQuery, CreateRoleRequest, UpdateRoleRequest, RoleResponse};
use crate::utils::permission::{verify_permission, check_is_admin};
use crate::common::constants::permission::{SystemModule, SystemAction};

/// Chuyển đổi struct model Role sang RoleResponse DTO để trả về Client
fn to_response(role: Role) -> RoleResponse {
    let permissions_val: serde_json::Value = serde_json::from_str(&role.permissions)
        .unwrap_or_else(|_| serde_json::json!({}));
    RoleResponse {
        id: role.id,
        created_date: role.created_date,
        created_by: role.created_by,
        updated_date: role.updated_date,
        updated_by: role.updated_by,
        version: role.version,
        code: role.code,
        name: role.name,
        description: role.description,
        permissions: permissions_val,
    }
}

/// API GET /roles/permissions-template - Lấy cây sơ đồ quyền hạn mặc định hệ thống
#[get("/api/roles/permissions-template")]
pub async fn get_permissions_template(
    _user: UserClaims,
) -> impl Responder {
    HttpResponse::Ok().json(crate::common::constants::permission::get_default_role_group())
}

/// API GET /roles - Danh sách vai trò phân trang (Áp dụng RBAC & Tenant)
#[get("/api/roles")]
pub async fn get_roles(
    db: web::Data<PgPool>,
    query: web::Query<RoleQuery>,
    user: UserClaims,
) -> impl Responder {
    // 1. Kiểm tra quyền hạn vai trò (RBAC) - Yêu cầu quyền "view" trong module "roles"
    match verify_permission(db.get_ref(), user.user_id, SystemModule::Roles, SystemAction::View).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền xem thông tin vai trò"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    let is_admin = check_is_admin(db.get_ref(), user.user_id).await.unwrap_or(false);
    let limit = query.size.unwrap_or(10).clamp(1, 100);
    let offset = query.page.unwrap_or(1).saturating_sub(1) * limit;
    
    let name_pattern = query.name.as_ref().map(|x| format!("%{}%", x));
    let code_pattern = query.code.as_ref().map(|x| format!("%{}%", x));

    // 2. Tính tổng số bản ghi (Áp dụng phân quyền Tenant)
    // - Admin: Xem toàn bộ Roles trong hệ thống
    // - Người dùng thường (Tenant): Chỉ xem Roles do chính mình tạo ra (created_by == user.user_id)
    let total = if is_admin {
        match sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!" 
            FROM roles 
            WHERE is_deleted = false 
              AND ($1::varchar IS NULL OR name ILIKE $1) 
              AND ($2::varchar IS NULL OR code ILIKE $2)
            "#,
            name_pattern,
            code_pattern
        )
        .fetch_one(db.get_ref())
        .await {
            Ok(r) => r.count,
            Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "Lỗi truy vấn tổng lượng vai trò"
            })),
        }
    } else {
        match sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!" 
            FROM roles 
            WHERE is_deleted = false 
              AND created_by = $1
              AND ($2::varchar IS NULL OR name ILIKE $2) 
              AND ($3::varchar IS NULL OR code ILIKE $3)
            "#,
            user.user_id,
            name_pattern,
            code_pattern
        )
        .fetch_one(db.get_ref())
        .await {
            Ok(r) => r.count,
            Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "Lỗi truy vấn tổng lượng vai trò"
            })),
        }
    };

    // 3. Lấy dữ liệu danh sách phân trang (Áp dụng phân quyền Tenant)
    let roles_res = if is_admin {
        sqlx::query_as::<_, Role>(
            r#"
            SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, name, description, permissions
            FROM roles
            WHERE is_deleted = false 
              AND ($1::varchar IS NULL OR name ILIKE $1) 
              AND ($2::varchar IS NULL OR code ILIKE $2)
            ORDER BY created_date DESC
            LIMIT $3 OFFSET $4
            "#
        )
        .bind(name_pattern)
        .bind(code_pattern)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(db.get_ref())
        .await
    } else {
        sqlx::query_as::<_, Role>(
            r#"
            SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, name, description, permissions
            FROM roles
            WHERE is_deleted = false 
              AND created_by = $1
              AND ($2::varchar IS NULL OR name ILIKE $2) 
              AND ($3::varchar IS NULL OR code ILIKE $3)
            ORDER BY created_date DESC
            LIMIT $4 OFFSET $5
            "#
        )
        .bind(user.user_id)
        .bind(name_pattern)
        .bind(code_pattern)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(db.get_ref())
        .await
    };

    match roles_res {
        Ok(items) => {
            let responses: Vec<RoleResponse> = items.into_iter().map(to_response).collect();
            HttpResponse::Ok().json(serde_json::json!({
                "items": responses,
                "total": total,
                "page": query.page.unwrap_or(1),
                "size": limit,
            }))
        }
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi tải danh sách vai trò"
        })),
    }
}

/// API GET /roles/{id} - Xem chi tiết vai trò cụ thể
#[get("/api/roles/{id}")]
pub async fn get_role_detail(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
    user: UserClaims,
) -> impl Responder {
    let role_id = path.into_inner();

    // 1. Kiểm tra quyền hạn vai trò (RBAC) - Yêu cầu quyền "view" trong module "roles"
    match verify_permission(db.get_ref(), user.user_id, SystemModule::Roles, SystemAction::View).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền xem thông tin vai trò"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    let is_admin = check_is_admin(db.get_ref(), user.user_id).await.unwrap_or(false);

    // 2. Tìm kiếm vai trò
    let role_res = sqlx::query_as::<_, Role>(
        r#"
        SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, name, description, permissions
        FROM roles
        WHERE id = $1 AND is_deleted = false
        "#
    )
    .bind(role_id)
    .fetch_optional(db.get_ref())
    .await;

    match role_res {
        Ok(Some(role)) => {
            // 3. Phân quyền Tenant: người dùng thường chỉ được xem vai trò do chính họ tạo ra
            if !is_admin && role.created_by != Some(user.user_id) {
                return HttpResponse::Forbidden().json(serde_json::json!({
                    "message": "Bạn không có quyền xem vai trò này"
                }));
            }
            HttpResponse::Ok().json(to_response(role))
        }
        Ok(None) => HttpResponse::NotFound().json(serde_json::json!({
            "message": "Không tìm thấy vai trò"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi truy vấn chi tiết vai trò"
        })),
    }
}

/// API POST /roles - Thêm mới vai trò và gán sơ đồ quyền
#[post("/api/roles")]
pub async fn create_role(
    db: web::Data<PgPool>,
    req: web::Json<CreateRoleRequest>,
    user: UserClaims,
) -> impl Responder {
    // 1. Kiểm tra quyền RBAC - Yêu cầu quyền "add" trong module "roles"
    match verify_permission(db.get_ref(), user.user_id, SystemModule::Roles, SystemAction::Add).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền thêm vai trò"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    let code = req.code.trim().to_string();
    let name = req.name.trim().to_string();

    if code.is_empty() || name.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Mã và tên vai trò không được để trống"
        }));
    }

    // 2. Kiểm tra trùng lặp code hoặc name
    let exists_res = sqlx::query!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM roles
            WHERE is_deleted = false AND (code = $1 OR name = $2)
        ) as "exists!"
        "#,
        code,
        name
    )
    .fetch_one(db.get_ref())
    .await;

    match exists_res {
        Ok(r) if r.exists => return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Mã hoặc tên vai trò đã tồn tại trong hệ thống"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra trùng lặp vai trò"
        })),
        _ => {}
    }

    // 3. Chuyển đổi permissions JSON thành String
    let permissions_str = req.permissions.to_string();
    let new_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    // 4. Thực hiện thêm mới vào cơ sở dữ liệu
    let insert_res = sqlx::query_as::<_, Role>(
        r#"
        INSERT INTO roles (id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, name, description, permissions)
        VALUES ($1, $2, $3, $4, $5, 1, false, $6, $7, $8, $9)
        RETURNING id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, name, description, permissions
        "#
    )
    .bind(new_id)
    .bind(now)
    .bind(user.user_id)
    .bind(now)
    .bind(user.user_id)
    .bind(code)
    .bind(name)
    .bind(&req.description)
    .bind(permissions_str)
    .fetch_one(db.get_ref())
    .await;

    match insert_res {
        Ok(role) => HttpResponse::Created().json(to_response(role)),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi tạo mới vai trò"
        })),
    }
}

/// API PUT /roles/{id} - Cập nhật thông tin vai trò & sơ đồ quyền hạn
#[put("/api/roles/{id}")]
pub async fn update_role(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
    req: web::Json<UpdateRoleRequest>,
    user: UserClaims,
) -> impl Responder {
    let role_id = path.into_inner();

    // 1. Kiểm tra quyền RBAC - Yêu cầu quyền "edit" trong module "roles"
    match verify_permission(db.get_ref(), user.user_id, SystemModule::Roles, SystemAction::Edit).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền sửa vai trò"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    let is_admin = check_is_admin(db.get_ref(), user.user_id).await.unwrap_or(false);

    // 2. Lấy thông tin vai trò hiện tại để kiểm tra
    let current_role = match sqlx::query_as::<_, Role>(
        r#"
        SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, name, description, permissions
        FROM roles
        WHERE id = $1 AND is_deleted = false
        "#
    )
    .bind(role_id)
    .fetch_optional(db.get_ref())
    .await {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({
            "message": "Không tìm thấy vai trò cần cập nhật"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra vai trò"
        })),
    };

    // 3. Kiểm tra quyền Tenant: người dùng thường chỉ được sửa vai trò do chính họ tạo ra
    if !is_admin && current_role.created_by != Some(user.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền chỉnh sửa vai trò này"
        }));
    }

    let name = req.name.trim().to_string();
    if name.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Tên vai trò không được để trống"
        }));
    }

    // 4. Kiểm tra trùng tên với vai trò khác
    let name_exists = sqlx::query!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM roles
            WHERE is_deleted = false AND name = $1 AND id <> $2
        ) as "exists!"
        "#,
        name,
        role_id
    )
    .fetch_one(db.get_ref())
    .await;

    match name_exists {
        Ok(r) if r.exists => return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Tên vai trò đã tồn tại trong hệ thống"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra trùng lặp tên vai trò"
        })),
        _ => {}
    }

    let permissions_str = req.permissions.to_string();
    let now = Utc::now().naive_utc();

    // 5. Cập nhật vai trò (sử dụng Optimistic Locking qua version)
    let update_res = sqlx::query_as::<_, Role>(
        r#"
        UPDATE roles
        SET name = $1,
            description = $2,
            permissions = $3,
            updated_date = $4,
            updated_by = $5,
            version = version + 1
        WHERE id = $6 AND version = $7 AND is_deleted = false
        RETURNING id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, name, description, permissions
        "#
    )
    .bind(name)
    .bind(&req.description)
    .bind(permissions_str)
    .bind(now)
    .bind(user.user_id)
    .bind(role_id)
    .bind(current_role.version)
    .fetch_optional(db.get_ref())
    .await;

    match update_res {
        Ok(Some(updated_role)) => HttpResponse::Ok().json(to_response(updated_role)),
        Ok(None) => HttpResponse::Conflict().json(serde_json::json!({
            "message": "Dữ liệu đã bị thay đổi bởi người dùng khác hoặc không tìm thấy, vui lòng tải lại trang"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi cập nhật vai trò"
        })),
    }
}

/// API DELETE /roles/{id} - Xóa mềm vai trò
#[delete("/api/roles/{id}")]
pub async fn delete_role(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
    user: UserClaims,
) -> impl Responder {
    let role_id = path.into_inner();

    // 1. Kiểm tra quyền RBAC - Yêu cầu quyền "deactive" trong module "roles"
    match verify_permission(db.get_ref(), user.user_id, SystemModule::Roles, SystemAction::DeActive).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền ngưng hoạt động/xóa vai trò"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    let is_admin = check_is_admin(db.get_ref(), user.user_id).await.unwrap_or(false);

    // 2. Kiểm tra vai trò tồn tại
    let current_role = match sqlx::query_as::<_, Role>(
        r#"
        SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, name, description, permissions
        FROM roles
        WHERE id = $1 AND is_deleted = false
        "#
    )
    .bind(role_id)
    .fetch_optional(db.get_ref())
    .await {
        Ok(Some(r)) => r,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({
            "message": "Không tìm thấy vai trò cần xóa"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra vai trò"
        })),
    };

    // 3. Kiểm tra quyền Tenant: người dùng thường chỉ được xóa vai trò do chính họ tạo ra
    if !is_admin && current_role.created_by != Some(user.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền xóa vai trò này"
        }));
    }

    // 4. Ràng buộc quan trọng: Không được xóa vai trò ADMIN hệ thống (code = ROLE_ADMIN) để tránh khóa hệ thống
    if current_role.code == "ROLE_ADMIN" {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Không thể xóa vai trò quản trị viên tối cao (ROLE_ADMIN) của hệ thống"
        }));
    }

    let now = Utc::now().naive_utc();

    // 5. Thực hiện xóa mềm
    let delete_res = sqlx::query!(
        r#"
        UPDATE roles
        SET is_deleted = true,
            updated_date = $1,
            updated_by = $2,
            version = version + 1
        WHERE id = $3 AND is_deleted = false
        "#,
        now,
        user.user_id,
        role_id
    )
    .execute(db.get_ref())
    .await;

    match delete_res {
        Ok(r) if r.rows_affected() > 0 => HttpResponse::Ok().json(serde_json::json!({
            "message": "Xóa vai trò thành công"
        })),
        Ok(_) => HttpResponse::NotFound().json(serde_json::json!({
            "message": "Không tìm thấy vai trò cần xóa hoặc đã bị xóa"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi xóa vai trò"
        })),
    }
}
