use sqlx::PgPool;
use uuid::Uuid;
use crate::common::constants::permission::{SystemModule, SystemAction};

/// Kiểm tra xem người dùng có phải là Admin hệ thống hay không (ROLE_ADMIN)
pub async fn check_is_admin(db: &PgPool, user_id: Uuid) -> Result<bool, sqlx::Error> {
    let row = sqlx::query!(
        r#"
        SELECT EXISTS (
            SELECT 1 FROM users u
            WHERE u.id = $1 AND u.is_admin = true AND u.is_deleted = false
        ) as "exists!"
        "#,
        user_id
    )
    .fetch_one(db)
    .await?;

    Ok(row.exists)
}

/// Kiểm tra xem người dùng có quyền cụ thể trong một module/action hay không
/// 
/// # Arguments
/// * `module` - Enum phân hệ cấp cao (ví dụ: SystemModule::TelegramBots)
/// * `action` - Enum hành động (ví dụ: SystemAction::View)
pub async fn verify_permission(
    db: &PgPool,
    user_id: Uuid,
    module: SystemModule,
    action: SystemAction,
) -> Result<bool, sqlx::Error> {
    let module_code = module.code();
    let action_code = action.field_key();


    // 1. Nếu là Admin hệ thống thì luôn được cấp mọi quyền
    if check_is_admin(db, user_id).await? {
        return Ok(true);
    }

    // 2. Truy vấn danh sách chuỗi permissions JSON từ các vai trò đang hoạt động của người dùng
    let roles = sqlx::query!(
        r#"
        SELECT r.permissions as "permissions!"
        FROM user_roles ur
        JOIN roles r ON ur.role_id = r.id
        WHERE ur.user_id = $1 AND ur.is_deleted = false AND r.is_deleted = false
        "#,
        user_id
    )
    .fetch_all(db)
    .await?;

    for role in roles {
        if let Ok(group) = serde_json::from_str::<serde_json::Value>(&role.permissions) {
            // Xác định module cần kiểm tra (ví dụ: group["telegramBots"])
            // Lưu ý: serde rename_all = "camelCase" nên ta tìm theo camelCase hoặc snake_case để tránh lỗi đặt tên
            let camel_module = to_camel_case(module_code);
            let module = group.get(module_code).or_else(|| group.get(&camel_module));

            if let Some(module) = module {
                // Kiểm tra xem có quyền "all" (giá trị true) ở cấp cao nhất không
                if let Some(all_action) = module.get("all") {
                    if all_action.get("value").and_then(|v| v.as_bool()) == Some(true) {
                        return Ok(true);
                    }
                }

                // Kiểm tra trong các PermissionItem con (children)
                if let Some(children) = module.get("children").and_then(|c| c.as_array()) {
                    for child in children {
                        // Kiểm tra quyền "all" của phần tử con
                        if let Some(all_action) = child.get("all") {
                            if all_action.get("value").and_then(|v| v.as_bool()) == Some(true) {
                                return Ok(true);
                            }
                        }

                        // Kiểm tra quyền cụ thể (ví dụ: view, add, edit, delete)
                        if let Some(action) = child.get(action_code) {
                            if action.get("value").and_then(|v| v.as_bool()) == Some(true) {
                                return Ok(true);
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(false)
}

/// Helper chuyển đổi snake_case sang camelCase
fn to_camel_case(s: &str) -> String {
    let mut camel = String::new();
    let mut capitalize_next = false;
    for c in s.chars() {
        if c == '_' {
            capitalize_next = true;
        } else if capitalize_next {
            camel.push(c.to_ascii_uppercase());
            capitalize_next = false;
        } else {
            camel.push(c);
        }
    }
    camel
}
