use actix_web::{get, post, put, web, HttpResponse, Responder};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;

use crate::middlewares::auth::UserClaims;
use crate::models::telegram_bot::{TelegramBot, BotStatus};
use crate::dtos::telegram_bot::{BotQuery, CreateBotRequest, UpdateBotRequest, BotResponse};
use crate::utils::permission::{verify_permission, check_is_admin};
use crate::services::telegram::TelegramService;
use crate::common::constants::permission::{SystemModule, SystemAction};

/// Chuyển đổi struct model TelegramBot sang BotResponse DTO để trả về Client
fn to_response(bot: TelegramBot) -> BotResponse {
    BotResponse {
        id: bot.id,
        created_date: bot.created_date,
        created_by: bot.created_by,
        updated_date: bot.updated_date,
        updated_by: bot.updated_by,
        version: bot.version,
        code: bot.code,
        bot_username: bot.bot_username,
        status: bot.status,
        total_files_handled: bot.total_files_handled,
    }
}

/// Helper khởi tạo TelegramService cấu hình proxy từ biến môi trường
fn init_telegram_service() -> TelegramService {
    let proxy = std::env::var("TELEGRAM_PROXY").ok();
    TelegramService::new(proxy)
        .unwrap_or_else(|_| TelegramService::new(None).expect("Không thể tạo TelegramService"))
}

/// API GET /telegram-bots - Danh sách phân trang Bot lưu trữ lưu trữ
#[get("/api/telegram-bots")]
pub async fn get_bots(
    db: web::Data<PgPool>,
    query: web::Query<BotQuery>,
    user: UserClaims,
) -> impl Responder {
    // 1. Kiểm tra quyền hạn vai trò (RBAC) - Yêu cầu quyền "view" trong module "telegram_bots"
    match verify_permission(db.get_ref(), user.user_id, SystemModule::TelegramBots, SystemAction::View).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền xem thông tin Bot lưu trữ Telegram"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    let is_admin = check_is_admin(db.get_ref(), user.user_id).await.unwrap_or(false);
    let limit = query.size.unwrap_or(10).clamp(1, 100);
    let offset = query.page.unwrap_or(1).saturating_sub(1) * limit;
    let bot_username_pattern = query.bot_username.as_ref().map(|x| format!("%{}%", x));

    // 2. Tính tổng số bản ghi (Áp dụng phân quyền Tenant)
    // - Admin: Xem toàn bộ Bot trong hệ thống
    // - Người dùng thường (Tenant): Chỉ xem Bot do chính mình tạo ra (created_by == user.user_id)
    let total = if is_admin {
        match sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!" 
            FROM telegram_bots 
            WHERE is_deleted = false 
              AND ($1::varchar IS NULL OR bot_username ILIKE $1) 
              AND ($2::varchar IS NULL OR status = $2)
            "#,
            bot_username_pattern,
            query.status
        )
        .fetch_one(db.get_ref())
        .await {
            Ok(r) => r.count,
            Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "Lỗi truy vấn tổng lượng Bot"
            })),
        }
    } else {
        match sqlx::query!(
            r#"
            SELECT COUNT(*) as "count!" 
            FROM telegram_bots 
            WHERE is_deleted = false 
              AND created_by = $1
              AND ($2::varchar IS NULL OR bot_username ILIKE $2) 
              AND ($3::varchar IS NULL OR status = $3)
            "#,
            user.user_id,
            bot_username_pattern,
            query.status
        )
        .fetch_one(db.get_ref())
        .await {
            Ok(r) => r.count,
            Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
                "message": "Lỗi truy vấn tổng lượng Bot"
            })),
        }
    };

    // 3. Lấy dữ liệu danh sách phân trang (Áp dụng phân quyền Tenant)
    let bots_res = if is_admin {
        sqlx::query_as::<_, TelegramBot>(
            r#"
            SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, bot_username, bot_token, status, total_files_handled
            FROM telegram_bots
            WHERE is_deleted = false 
              AND ($1::varchar IS NULL OR bot_username ILIKE $1) 
              AND ($2::varchar IS NULL OR status = $2)
            ORDER BY created_date DESC
            LIMIT $3 OFFSET $4
            "#
        )
        .bind(bot_username_pattern)
        .bind(&query.status)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(db.get_ref())
        .await
    } else {
        sqlx::query_as::<_, TelegramBot>(
            r#"
            SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, bot_username, bot_token, status, total_files_handled
            FROM telegram_bots
            WHERE is_deleted = false 
              AND created_by = $1
              AND ($2::varchar IS NULL OR bot_username ILIKE $2) 
              AND ($3::varchar IS NULL OR status = $3)
            ORDER BY created_date DESC
            LIMIT $4 OFFSET $5
            "#
        )
        .bind(user.user_id)
        .bind(bot_username_pattern)
        .bind(&query.status)
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(db.get_ref())
        .await
    };

    match bots_res {
        Ok(items) => {
            let responses: Vec<BotResponse> = items.into_iter().map(to_response).collect();
            HttpResponse::Ok().json(serde_json::json!({
                "items": responses,
                "total": total,
                "page": query.page.unwrap_or(1),
                "size": limit,
            }))
        }
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi tải danh sách Bot"
        })),
    }
}

/// API GET /telegram-bots/{id} - Xem chi tiết Bot lưu trữ Telegram
#[get("/api/telegram-bots/{id}")]
pub async fn get_bot_detail(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
    user: UserClaims,
) -> impl Responder {
    let bot_id = path.into_inner();

    // 1. Kiểm tra quyền hạn vai trò (RBAC)
    match verify_permission(db.get_ref(), user.user_id, SystemModule::TelegramBots, SystemAction::View).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền xem chi tiết Bot Telegram"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    // 2. Lấy thông tin Bot từ Database
    let bot_res = sqlx::query_as::<_, TelegramBot>(
        "SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, bot_username, bot_token, status, total_files_handled FROM telegram_bots WHERE id = $1 AND is_deleted = false"
    )
    .bind(bot_id)
    .fetch_optional(db.get_ref())
    .await;

    let bot = match bot_res {
        Ok(Some(b)) => b,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({
            "message": "Không tìm thấy Bot lưu trữ Telegram yêu cầu"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi cơ sở dữ liệu"
        })),
    };

    // 3. Kiểm tra Tenant Isolation (Người dùng thường chỉ được xem Bot do chính họ cấu hình)
    let is_admin = check_is_admin(db.get_ref(), user.user_id).await.unwrap_or(false);
    if !is_admin && bot.created_by != Some(user.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Yêu cầu bị từ chối: Bạn không có quyền truy cập tài nguyên của Tenant khác"
        }));
    }

    HttpResponse::Ok().json(to_response(bot))
}

/// API POST /telegram-bots - Tạo mới Bot lưu trữ lưu trữ (Có xác minh token thời gian thực)
#[post("/api/telegram-bots")]
pub async fn create_bot(
    db: web::Data<PgPool>,
    body: web::Json<CreateBotRequest>,
    user: UserClaims,
) -> impl Responder {
    let request = body.into_inner();

    // 1. Kiểm tra quyền hạn vai trò (RBAC)
    match verify_permission(db.get_ref(), user.user_id, SystemModule::TelegramBots, SystemAction::Add).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền thêm mới Bot lưu trữ Telegram"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    // Ràng buộc dữ liệu đầu vào
    if request.code.trim().is_empty() || request.bot_token.trim().is_empty() || request.bot_username.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Các thông tin code, username và token của Bot không được để trống"
        }));
    }

    // 2. Kiểm tra trùng lặp mã Code hoặc Username
    let exists_res = sqlx::query!(
        "SELECT EXISTS(SELECT 1 FROM telegram_bots WHERE (code = $1 OR bot_username = $2) AND is_deleted = false) as \"exists!\"",
        request.code,
        request.bot_username
    )
    .fetch_one(db.get_ref())
    .await;

    match exists_res {
        Ok(r) if r.exists => return HttpResponse::Conflict().json(serde_json::json!({
            "message": "Mã định danh Code hoặc Tên Username của Bot đã tồn tại trong hệ thống"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi xác minh trùng lặp dữ liệu"
        })),
        _ => {}
    }

    // 3. Premium Check: Kết nối thời gian thực đến Telegram Bot API để xác minh tính chính xác của Bot Token
    let telegram_service = init_telegram_service();
    match telegram_service.get_me(&request.bot_token).await {
        Ok(tg_user) => {
            // Xác thực xem username bot nhập vào có trùng khớp với dữ liệu thực từ Telegram không
            if let Some(ref tg_username) = tg_user.username {
                if tg_username.to_lowercase() != request.bot_username.to_lowercase() {
                    return HttpResponse::BadRequest().json(serde_json::json!({
                        "message": format!(
                            "Token hợp lệ nhưng thuộc về Bot @{} thay vì @{} như đăng ký",
                            tg_username, request.bot_username
                        )
                    }));
                }
            }
        }
        Err(_) => {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "message": "Không thể kết nối hoặc Token Bot không hợp lệ"
            }));
        }
    }

    // 4. Lưu thông tin vào Database (Bot được mặc định thuộc sở hữu của User/Tenant khởi tạo)
    let bot_id = Uuid::new_v4();
    let now = Utc::now().naive_utc();

    let insert_res = sqlx::query(
        r#"
        INSERT INTO telegram_bots (id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, bot_username, bot_token, status, total_files_handled)
        VALUES ($1, $2, $3, $4, $5, 1, false, $6, $7, $8, $9, 0)
        "#
    )
    .bind(bot_id)
    .bind(now)
    .bind(user.user_id)
    .bind(now)
    .bind(user.user_id)
    .bind(&request.code)
    .bind(&request.bot_username)
    .bind(&request.bot_token)
    .bind("ACTIVE") // Bot ban đầu ở trạng thái hoạt động
    .execute(db.get_ref())
    .await;

    match insert_res {
        Ok(_) => {
            // Trả về đối tượng vừa tạo
            let created_bot = TelegramBot {
                id: bot_id,
                created_date: now,
                created_by: Some(user.user_id),
                updated_date: now,
                updated_by: Some(user.user_id),
                version: 1,
                is_deleted: false,
                code: request.code,
                bot_username: request.bot_username,
                bot_token: request.bot_token,
                status: BotStatus::Active,
                total_files_handled: 0,
            };
            HttpResponse::Created().json(to_response(created_bot))
        }
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi khi lưu thông tin Bot mới"
        })),
    }
}

/// API PUT /telegram-bots/{id} - Cập nhật cấu hình Bot lưu trữ Telegram
#[put("/api/telegram-bots/{id}")]
pub async fn update_bot(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
    body: web::Json<UpdateBotRequest>,
    user: UserClaims,
) -> impl Responder {
    let bot_id = path.into_inner();
    let request = body.into_inner();

    // 1. Kiểm tra quyền hạn vai trò (RBAC)
    match verify_permission(db.get_ref(), user.user_id, SystemModule::TelegramBots, SystemAction::Edit).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền chỉnh sửa cấu hình Bot Telegram"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    // 2. Lấy thông tin Bot hiện có
    let bot_res = sqlx::query_as::<_, TelegramBot>(
        "SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, bot_username, bot_token, status, total_files_handled FROM telegram_bots WHERE id = $1 AND is_deleted = false"
    )
    .bind(bot_id)
    .fetch_optional(db.get_ref())
    .await;

    let bot = match bot_res {
        Ok(Some(b)) => b,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({
            "message": "Không tìm thấy Bot lưu trữ Telegram cần cập nhật"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi cơ sở dữ liệu"
        })),
    };

    // 3. Kiểm tra Tenant Isolation (Người dùng thường chỉ được sửa Bot thuộc sở hữu của mình)
    let is_admin = check_is_admin(db.get_ref(), user.user_id).await.unwrap_or(false);
    if !is_admin && bot.created_by != Some(user.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Yêu cầu bị từ chối: Bạn không có quyền chỉnh sửa tài nguyên của Tenant khác"
        }));
    }

    // 4. Nếu có cập nhật Token, thực hiện xác thực kết nối trước khi lưu
    let final_token = request.bot_token.unwrap_or_else(|| bot.bot_token.clone());
    if final_token != bot.bot_token {
        let telegram_service = init_telegram_service();
        if let Err(_) = telegram_service.get_me(&final_token).await {
            return HttpResponse::BadRequest().json(serde_json::json!({
                "message": "Không thể cập nhật cấu hình: Token Bot mới không hoạt động"
            }));
        }
    }

    // 5. Cập nhật thông tin vào DB
    let now = Utc::now().naive_utc();
    let update_res = sqlx::query(
        r#"
        UPDATE telegram_bots 
        SET bot_username = $1, bot_token = $2, updated_date = $3, updated_by = $4, version = version + 1
        WHERE id = $5 AND is_deleted = false
        "#
    )
    .bind(&request.bot_username)
    .bind(&final_token)
    .bind(now)
    .bind(user.user_id)
    .bind(bot_id)
    .execute(db.get_ref())
    .await;

    match update_res {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Cập nhật thông tin cấu hình Bot Telegram thành công!"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi cập nhật dữ liệu"
        })),
    }
}

/// API POST /telegram-bots/{id}/deactivate - Ngưng hoạt động Bot lưu trữ lưu trữ
#[post("/api/telegram-bots/{id}/deactivate")]
pub async fn deactivate_bot(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
    user: UserClaims,
) -> impl Responder {
    let bot_id = path.into_inner();

    // 1. Kiểm tra quyền hạn vai trò (RBAC)
    match verify_permission(db.get_ref(), user.user_id, SystemModule::TelegramBots, SystemAction::DeActive).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền ngưng hoạt động Bot Telegram"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    // 2. Lấy thông tin Bot hiện có
    let bot_res = sqlx::query_as::<_, TelegramBot>(
        "SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, bot_username, bot_token, status, total_files_handled FROM telegram_bots WHERE id = $1 AND is_deleted = false"
    )
    .bind(bot_id)
    .fetch_optional(db.get_ref())
    .await;

    let bot = match bot_res {
        Ok(Some(b)) => b,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({
            "message": "Không tìm thấy Bot lưu trữ Telegram yêu cầu"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi cơ sở dữ liệu"
        })),
    };

    // 3. Kiểm tra Tenant Isolation (Người dùng thường chỉ được thao tác trên Bot thuộc sở hữu của mình)
    let is_admin = check_is_admin(db.get_ref(), user.user_id).await.unwrap_or(false);
    if !is_admin && bot.created_by != Some(user.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Yêu cầu bị từ chối: Bạn không có quyền thao tác trên tài nguyên của Tenant khác"
        }));
    }

    // 4. Cập nhật trạng thái thành BANNED (Ngưng hoạt động)
    let now = Utc::now().naive_utc();
    let update_res = sqlx::query(
        r#"
        UPDATE telegram_bots 
        SET status = $1, updated_date = $2, updated_by = $3, version = version + 1
        WHERE id = $4 AND is_deleted = false
        "#
    )
    .bind("BANNED") // Tương ứng BotStatus::Banned
    .bind(now)
    .bind(user.user_id)
    .bind(bot_id)
    .execute(db.get_ref())
    .await;

    match update_res {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Đã dừng hoạt động của Bot lưu trữ Telegram thành công!"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi cập nhật trạng thái"
        })),
    }
}

/// API POST /telegram-bots/{id}/activate - Kích hoạt lại hoạt động Bot lưu trữ lưu trữ
#[post("/api/telegram-bots/{id}/activate")]
pub async fn activate_bot(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
    user: UserClaims,
) -> impl Responder {
    let bot_id = path.into_inner();

    // 1. Kiểm tra quyền hạn vai trò (RBAC)
    match verify_permission(db.get_ref(), user.user_id, SystemModule::TelegramBots, SystemAction::Active).await {
        Ok(true) => {}
        Ok(false) => return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Bạn không có quyền kích hoạt lại Bot Telegram"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi kiểm tra quyền"
        })),
    }

    // 2. Lấy thông tin Bot hiện có
    let bot_res = sqlx::query_as::<_, TelegramBot>(
        "SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, bot_username, bot_token, status, total_files_handled FROM telegram_bots WHERE id = $1 AND is_deleted = false"
    )
    .bind(bot_id)
    .fetch_optional(db.get_ref())
    .await;

    let bot = match bot_res {
        Ok(Some(b)) => b,
        Ok(None) => return HttpResponse::NotFound().json(serde_json::json!({
            "message": "Không tìm thấy Bot lưu trữ Telegram yêu cầu"
        })),
        Err(_) => return HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi cơ sở dữ liệu"
        })),
    };

    // 3. Kiểm tra Tenant Isolation
    let is_admin = check_is_admin(db.get_ref(), user.user_id).await.unwrap_or(false);
    if !is_admin && bot.created_by != Some(user.user_id) {
        return HttpResponse::Forbidden().json(serde_json::json!({
            "message": "Yêu cầu bị từ chối: Bạn không có quyền thao tác trên tài nguyên của Tenant khác"
        }));
    }

    // 4. Cập nhật trạng thái thành ACTIVE (Hoạt động lại)
    let now = Utc::now().naive_utc();
    let update_res = sqlx::query(
        r#"
        UPDATE telegram_bots 
        SET status = $1, updated_date = $2, updated_by = $3, version = version + 1
        WHERE id = $4 AND is_deleted = false
        "#
    )
    .bind("ACTIVE") // Tương ứng BotStatus::Active
    .bind(now)
    .bind(user.user_id)
    .bind(bot_id)
    .execute(db.get_ref())
    .await;

    match update_res {
        Ok(_) => HttpResponse::Ok().json(serde_json::json!({
            "message": "Kích hoạt hoạt động lại cho Bot lưu trữ Telegram thành công!"
        })),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi cập nhật trạng thái"
        })),
    }
}
