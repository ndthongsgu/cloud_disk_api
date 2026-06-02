use actix_web::{get, post, delete, web, HttpResponse, Responder};
use sqlx::PgPool;
use uuid::Uuid;
use chrono::Utc;
use actix_multipart::Multipart;
use futures_util::StreamExt;
use serde::{Deserialize, Serialize};

use crate::middlewares::auth::UserClaims;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FilesQuery {
    pub parent_id: Option<String>,
    pub is_trashed: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateFolderRequest {
    pub name: String,
    pub parent_id: Option<String>,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DiskFileResponse {
    pub id: Uuid,
    pub name: String,
    pub r#type: String,
    pub size: i64,
    pub path: String,
    pub parent_id: Option<Uuid>,
    pub owner_id: Uuid,
    pub created_at: String,
    pub updated_at: String,
    pub is_starred: bool,
    pub is_trashed: bool,
    pub url: Option<String>,
}

fn get_file_type(mime_type: &str) -> String {
    let mime = mime_type.to_lowercase();
    if mime.contains("image/") {
        "image".to_string()
    } else if mime.contains("video/") {
        "video".to_string()
    } else if mime.contains("audio/") {
        "audio".to_string()
    } else if mime.contains("pdf") {
        "pdf".to_string()
    } else if mime.contains("zip") || mime.contains("tar") || mime.contains("rar") || mime.contains("gz") {
        "archive".to_string()
    } else if mime.contains("text/") || mime.contains("document") || mime.contains("msword") || mime.contains("sheet") {
        "document".to_string()
    } else {
        "other".to_string()
    }
}

/// API GET /files - Lấy danh sách tệp tin và thư mục ảo trong thư mục hiện tại
#[get("/api/files")]
pub async fn get_files(
    db: web::Data<PgPool>,
    query: web::Query<FilesQuery>,
    user: UserClaims,
) -> impl Responder {
    let is_trashed = query.is_trashed.unwrap_or(false);
    
    // Phân tích cú pháp parent_id
    let parent_id = match &query.parent_id {
        Some(s) if s != "null" && !s.is_empty() => Uuid::parse_str(s).ok(),
        _ => None,
    };

    // 1. Lấy danh sách các thư mục ảo
    let folders_res = sqlx::query!(
        r#"
        SELECT id, folder_name, parent_folder_id, created_date, updated_date
        FROM folders
        WHERE user_id = $1 AND is_deleted = $2 AND (parent_folder_id = $3 OR ($3 IS NULL AND parent_folder_id IS NULL))
        ORDER BY folder_name ASC
        "#,
        user.user_id,
        is_trashed,
        parent_id
    )
    .fetch_all(db.get_ref())
    .await;

    // 2. Lấy danh sách các tệp tin ảo
    let files_res = sqlx::query!(
        r#"
        SELECT id, file_name, folder_id, size_bytes, mime_type, created_date, updated_date
        FROM files
        WHERE user_id = $1 AND is_deleted = $2 AND (folder_id = $3 OR ($3 IS NULL AND folder_id IS NULL))
        ORDER BY file_name ASC
        "#,
        user.user_id,
        is_trashed,
        parent_id
    )
    .fetch_all(db.get_ref())
    .await;

    let mut list = Vec::new();

    if let Ok(folders) = folders_res {
        for f in folders {
            list.push(DiskFileResponse {
                id: f.id,
                name: f.folder_name,
                r#type: "folder".to_string(),
                size: 0,
                path: "".to_string(),
                parent_id: f.parent_folder_id,
                owner_id: user.user_id,
                created_at: f.created_date.to_string(),
                updated_at: f.updated_date.to_string(),
                is_starred: false,
                is_trashed,
                url: None,
            });
        }
    }

    if let Ok(files) = files_res {
        for f in files {
            list.push(DiskFileResponse {
                id: f.id,
                name: f.file_name,
                r#type: get_file_type(&f.mime_type),
                size: f.size_bytes,
                path: "".to_string(),
                parent_id: f.folder_id,
                owner_id: user.user_id,
                created_at: f.created_date.to_string(),
                updated_at: f.updated_date.to_string(),
                is_starred: false,
                is_trashed,
                url: Some(format!("http://127.0.0.1:8080/api/files/{}/download", f.id)),
            });
        }
    }

    HttpResponse::Ok().json(list)
}

/// API GET /files/starred - Danh sách các mục được đánh dấu sao quan trọng (Simulated)
#[get("/api/files/starred")]
pub async fn get_starred_files(
    _db: web::Data<PgPool>,
    _user: UserClaims,
) -> impl Responder {
    let list: Vec<DiskFileResponse> = Vec::new();
    HttpResponse::Ok().json(list)
}

/// API GET /files/trash - Danh sách các mục đang nằm trong thùng rác
#[get("/api/files/trash")]
pub async fn get_trashed_files(
    db: web::Data<PgPool>,
    user: UserClaims,
) -> impl Responder {
    // 1. Truy vấn các thư mục bị xóa mềm
    let folders_res = sqlx::query!(
        r#"
        SELECT id, folder_name, parent_folder_id, created_date, updated_date
        FROM folders
        WHERE user_id = $1 AND is_deleted = true
        ORDER BY folder_name ASC
        "#,
        user.user_id
    )
    .fetch_all(db.get_ref())
    .await;

    // 2. Truy vấn các tệp tin bị xóa mềm
    let files_res = sqlx::query!(
        r#"
        SELECT id, file_name, folder_id, size_bytes, mime_type, created_date, updated_date
        FROM files
        WHERE user_id = $1 AND is_deleted = true
        ORDER BY file_name ASC
        "#,
        user.user_id
    )
    .fetch_all(db.get_ref())
    .await;

    let mut list = Vec::new();

    if let Ok(folders) = folders_res {
        for f in folders {
            list.push(DiskFileResponse {
                id: f.id,
                name: f.folder_name,
                r#type: "folder".to_string(),
                size: 0,
                path: "".to_string(),
                parent_id: f.parent_folder_id,
                owner_id: user.user_id,
                created_at: f.created_date.to_string(),
                updated_at: f.updated_date.to_string(),
                is_starred: false,
                is_trashed: true,
                url: None,
            });
        }
    }

    if let Ok(files) = files_res {
        for f in files {
            list.push(DiskFileResponse {
                id: f.id,
                name: f.file_name,
                r#type: get_file_type(&f.mime_type),
                size: f.size_bytes,
                path: "".to_string(),
                parent_id: f.folder_id,
                owner_id: user.user_id,
                created_at: f.created_date.to_string(),
                updated_at: f.updated_date.to_string(),
                is_starred: false,
                is_trashed: true,
                url: None,
            });
        }
    }

    HttpResponse::Ok().json(list)
}

/// API POST /files/folder - Tạo mới thư mục ảo lưu trữ
#[post("/api/files/folder")]
pub async fn create_folder(
    db: web::Data<PgPool>,
    body: web::Json<CreateFolderRequest>,
    user: UserClaims,
) -> impl Responder {
    let req = body.into_inner();
    if req.name.trim().is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Tên thư mục không được để trống"
        }));
    }

    let parent_id = match &req.parent_id {
        Some(s) if s != "null" && !s.is_empty() => Uuid::parse_str(s).ok(),
        _ => None,
    };

    let folder_id = Uuid::new_v4();
    let folder_code = format!("DIR-{}", &folder_id.to_string()[..8].to_uppercase());
    let now = Utc::now().naive_utc();

    let insert_res = sqlx::query(
        r#"
        INSERT INTO folders (id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, user_id, parent_folder_id, folder_name, color_code)
        VALUES ($1, $2, $3, $4, $5, 1, false, $6, $7, $8, $9, NULL)
        "#
    )
    .bind(folder_id)
    .bind(now)
    .bind(user.user_id)
    .bind(now)
    .bind(user.user_id)
    .bind(folder_code)
    .bind(user.user_id)
    .bind(parent_id)
    .bind(&req.name)
    .execute(db.get_ref())
    .await;

    match insert_res {
        Ok(_) => HttpResponse::Created().json(DiskFileResponse {
            id: folder_id,
            name: req.name,
            r#type: "folder".to_string(),
            size: 0,
            path: "".to_string(),
            parent_id,
            owner_id: user.user_id,
            created_at: now.to_string(),
            updated_at: now.to_string(),
            is_starred: false,
            is_trashed: false,
            url: None,
        }),
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi khi tạo thư mục mới"
        })),
    }
}

/// API GET /storage/info - Thống kê dung lượng lưu trữ của tài khoản
#[get("/api/storage/info")]
pub async fn get_storage_info(
    db: web::Data<PgPool>,
    user: UserClaims,
) -> impl Responder {
    let user_res = sqlx::query!(
        "SELECT storage_quota_bytes, used_storage_bytes FROM users WHERE id = $1 AND is_deleted = false",
        user.user_id
    )
    .fetch_optional(db.get_ref())
    .await;

    match user_res {
        Ok(Some(u)) => {
            let total = u.storage_quota_bytes.unwrap_or(15 * 1024 * 1024 * 1024); // Mặc định 15GB nếu NULL
            HttpResponse::Ok().json(serde_json::json!({
                "total": total,
                "used": u.used_storage_bytes
            }))
        }
        _ => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi khi truy vấn thông tin bộ nhớ"
        })),
    }
}

/// API DELETE /files/{id}/trash - Di chuyển tệp tin hoặc thư mục vào thùng rác (Xóa mềm)
#[delete("/api/files/{id}/trash")]
pub async fn trash_file(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
    user: UserClaims,
) -> impl Responder {
    let id = path.into_inner();
    let now = Utc::now().naive_utc();

    // 1. Thử cập nhật trong bảng folders trước
    let folder_update = sqlx::query(
        "UPDATE folders SET is_deleted = true, updated_date = $1, updated_by = $2 WHERE id = $3 AND user_id = $4 AND is_deleted = false"
    )
    .bind(now)
    .bind(user.user_id)
    .bind(id)
    .execute(db.get_ref())
    .await;

    if let Ok(res) = folder_update {
        if res.rows_affected() > 0 {
            return HttpResponse::Ok().json(serde_json::json!({ "message": "Đã di chuyển thư mục vào thùng rác thành công" }));
        }
    }

    // 2. Thử cập nhật trong bảng files
    let file_update = sqlx::query(
        "UPDATE files SET is_deleted = true, updated_date = $1, updated_by = $2 WHERE id = $3 AND user_id = $4 AND is_deleted = false"
    )
    .bind(now)
    .bind(user.user_id)
    .bind(id)
    .execute(db.get_ref())
    .await;

    match file_update {
        Ok(res) if res.rows_affected() > 0 => {
            HttpResponse::Ok().json(serde_json::json!({ "message": "Đã di chuyển tệp tin vào thùng rác thành công" }))
        }
        _ => HttpResponse::NotFound().json(serde_json::json!({ "message": "Không tìm thấy tệp hoặc thư mục yêu cầu" })),
    }
}

/// API POST /files/{id}/restore - Khôi phục tệp tin hoặc thư mục từ thùng rác
#[post("/api/files/{id}/restore")]
pub async fn restore_file(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
    user: UserClaims,
) -> impl Responder {
    let id = path.into_inner();
    let now = Utc::now().naive_utc();

    // 1. Thử khôi phục thư mục ảo
    let folder_update = sqlx::query(
        "UPDATE folders SET is_deleted = false, updated_date = $1, updated_by = $2 WHERE id = $3 AND user_id = $4 AND is_deleted = true"
    )
    .bind(now)
    .bind(user.user_id)
    .bind(id)
    .execute(db.get_ref())
    .await;

    if let Ok(res) = folder_update {
        if res.rows_affected() > 0 {
            return HttpResponse::Ok().json(serde_json::json!({ "message": "Đã khôi phục thư mục thành công" }));
        }
    }

    // 2. Thử khôi phục tệp tin ảo
    let file_update = sqlx::query(
        "UPDATE files SET is_deleted = false, updated_date = $1, updated_by = $2 WHERE id = $3 AND user_id = $4 AND is_deleted = true"
    )
    .bind(now)
    .bind(user.user_id)
    .bind(id)
    .execute(db.get_ref())
    .await;

    match file_update {
        Ok(res) if res.rows_affected() > 0 => {
            HttpResponse::Ok().json(serde_json::json!({ "message": "Đã khôi phục tệp tin thành công" }))
        }
        _ => HttpResponse::NotFound().json(serde_json::json!({ "message": "Không tìm thấy tệp hoặc thư mục yêu cầu" })),
    }
}

/// API DELETE /files/{id} - Xóa vĩnh viễn tệp tin hoặc thư mục ảo khỏi cơ sở dữ liệu
#[delete("/api/files/{id}")]
pub async fn delete_file_permanently(
    db: web::Data<PgPool>,
    path: web::Path<Uuid>,
    user: UserClaims,
) -> impl Responder {
    let id = path.into_inner();

    // 1. Thử xóa vĩnh viễn thư mục ảo
    let folder_delete = sqlx::query(
        "DELETE FROM folders WHERE id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(user.user_id)
    .execute(db.get_ref())
    .await;

    if let Ok(res) = folder_delete {
        if res.rows_affected() > 0 {
            return HttpResponse::Ok().json(serde_json::json!({ "message": "Đã xóa thư mục vĩnh viễn" }));
        }
    }

    // 2. Thử lấy thông tin dung lượng tệp tin ảo trước khi xóa để giảm định mức used_storage_bytes của tài khoản
    let file_info = sqlx::query!(
        "SELECT size_bytes FROM files WHERE id = $1 AND user_id = $2",
        id,
        user.user_id
    )
    .fetch_optional(db.get_ref())
    .await;

    let size_to_subtract = match file_info {
        Ok(Some(f)) => f.size_bytes,
        _ => 0,
    };

    // 3. Tiến hành xóa tệp tin ảo khỏi DB
    let file_delete = sqlx::query(
        "DELETE FROM files WHERE id = $1 AND user_id = $2"
    )
    .bind(id)
    .bind(user.user_id)
    .execute(db.get_ref())
    .await;

    match file_delete {
        Ok(res) if res.rows_affected() > 0 => {
            // Giảm bớt dung lượng tài khoản
            let _ = sqlx::query!(
                "UPDATE users SET used_storage_bytes = GREATEST(0, used_storage_bytes - $1) WHERE id = $2",
                size_to_subtract,
                user.user_id
            )
            .execute(db.get_ref())
            .await;

            HttpResponse::Ok().json(serde_json::json!({ "message": "Đã xóa tệp tin vĩnh viễn thành công" }))
        }
        _ => HttpResponse::NotFound().json(serde_json::json!({ "message": "Không tìm thấy tệp hoặc thư mục cần xóa vĩnh viễn" })),
    }
}

/// API POST /files/upload - Tải lên tệp tin ảo dạng Multipart Form
#[post("/api/files/upload")]
pub async fn upload_file(
    db: web::Data<PgPool>,
    mut payload: Multipart,
    user: UserClaims,
) -> impl Responder {
    let mut file_bytes = Vec::new();
    let mut filename = String::new();
    let mut mime_type = "application/octet-stream".to_string();
    let mut parent_id: Option<Uuid> = None;

    while let Some(item) = payload.next().await {
        let mut field = match item {
            Ok(f) => f,
            Err(_) => return HttpResponse::BadRequest().json(serde_json::json!({ "message": "Lỗi luồng dữ liệu truyền tải" })),
        };

        if let Some(content_disposition) = field.content_disposition() {
            let name = content_disposition.get_name().unwrap_or("");

            if name == "file" {
                if let Some(fname) = content_disposition.get_filename() {
                    filename = fname.to_string();
                }
                if let Some(mtype) = field.content_type() {
                    mime_type = mtype.to_string();
                }

                while let Some(chunk) = field.next().await {
                    let data = match chunk {
                        Ok(bytes) => bytes,
                        Err(_) => return HttpResponse::BadRequest().json(serde_json::json!({ "message": "Lỗi đọc phân đoạn dữ liệu" })),
                    };
                    file_bytes.extend_from_slice(&data);
                }
            } else if name == "parentId" {
                let mut value_bytes = Vec::new();
                while let Some(chunk) = field.next().await {
                    if let Ok(bytes) = chunk {
                        value_bytes.extend_from_slice(&bytes);
                    }
                }
                if let Ok(value_str) = String::from_utf8(value_bytes) {
                    if value_str != "null" && !value_str.trim().is_empty() {
                        parent_id = Uuid::parse_str(value_str.trim()).ok();
                    }
                }
            }
        }
    }

    if filename.is_empty() {
        return HttpResponse::BadRequest().json(serde_json::json!({
            "message": "Không tìm thấy dữ liệu tệp tin để lưu trữ"
        }));
    }

    let file_size = file_bytes.len() as i64;
    let file_id = Uuid::new_v4();
    let file_code = format!("FILE-{}", &file_id.to_string()[..8].to_uppercase());
    let now = Utc::now().naive_utc();
    let is_video = mime_type.contains("video/");

    // 1. Lưu thông tin tệp tin vào cơ sở dữ liệu ảo
    let insert_res = sqlx::query(
        r#"
        INSERT INTO files (id, created_date, created_by, updated_date, updated_by, version, is_deleted, code, user_id, folder_id, file_name, mime_type, size_bytes, is_video, upload_status)
        VALUES ($1, $2, $3, $4, $5, 1, false, $6, $7, $8, $9, $10, $11, $12, $13)
        "#
    )
    .bind(file_id)
    .bind(now)
    .bind(user.user_id)
    .bind(now)
    .bind(user.user_id)
    .bind(file_code)
    .bind(user.user_id)
    .bind(parent_id)
    .bind(&filename)
    .bind(&mime_type)
    .bind(file_size)
    .bind(is_video)
    .bind("COMPLETED")
    .execute(db.get_ref())
    .await;

    match insert_res {
        Ok(_) => {
            // Cập nhật dung lượng sử dụng cho người dùng
            let _ = sqlx::query!(
                "UPDATE users SET used_storage_bytes = used_storage_bytes + $1 WHERE id = $2",
                file_size,
                user.user_id
            )
            .execute(db.get_ref())
            .await;

            HttpResponse::Created().json(DiskFileResponse {
                id: file_id,
                name: filename,
                r#type: get_file_type(&mime_type),
                size: file_size,
                path: "".to_string(),
                parent_id,
                owner_id: user.user_id,
                created_at: now.to_string(),
                updated_at: now.to_string(),
                is_starred: false,
                is_trashed: false,
                url: Some(format!("http://127.0.0.1:8080/api/files/{}/download", file_id)),
            })
        }
        Err(_) => HttpResponse::InternalServerError().json(serde_json::json!({
            "message": "Lỗi lưu thông tin tệp tin vào CSDL"
        })),
    }
}
