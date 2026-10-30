use crate::models::user::UserStatus;
use crate::models::telegram_bot::BotStatus;
use uuid::Uuid;

// --- User Status Helpers ---

pub fn get_user_status_label(status: UserStatus) -> &'static str {
    match status {
        UserStatus::Active => "Hoạt động",
        UserStatus::Locked => "Tạm khóa",
        UserStatus::Banned => "Bị cấm",
    }
}

pub fn can_deactivate_user(status: UserStatus, is_deleted: bool, user_id: Uuid, current_user_id: Option<Uuid>) -> bool {
    if is_deleted {
        return false;
    }
    // Users cannot deactivate themselves
    if let Some(curr_id) = current_user_id {
        if user_id == curr_id {
            return false;
        }
    }
    matches!(status, UserStatus::Active)
}

pub fn can_activate_user(status: UserStatus, is_deleted: bool) -> bool {
    if is_deleted {
        return false;
    }
    matches!(status, UserStatus::Locked | UserStatus::Banned)
}

// --- Telegram Bot Status Helpers ---

pub fn get_bot_status_label(status: BotStatus) -> &'static str {
    match status {
        BotStatus::ACTIVE => "Hoạt động",
        BotStatus::RATELIMITED => "Bị giới hạn",
        BotStatus::BANNED => "Bị khóa",
    }
}

pub fn can_deactivate_bot(status: BotStatus, is_deleted: bool) -> bool {
    if is_deleted {
        return false;
    }
    matches!(status, BotStatus::ACTIVE | BotStatus::RATELIMITED)
}

pub fn can_activate_bot(status: BotStatus, is_deleted: bool) -> bool {
    if is_deleted {
        return false;
    }
    matches!(status, BotStatus::BANNED)
}

// --- Role Status & Actions ---

pub fn get_deleted_label(is_deleted: bool) -> &'static str {
    if is_deleted {
        "Đã xóa"
    } else {
        "Hoạt động"
    }
}

pub fn can_delete_role(code: &str, is_deleted: bool) -> bool {
    if is_deleted {
        return false;
    }
    code != "ROLE_ADMIN"
}

// --- File & Folder Actions ---

pub fn can_trash_file(is_trashed: bool) -> bool {
    !is_trashed
}

pub fn can_restore_file(is_trashed: bool) -> bool {
    is_trashed
}

pub fn can_delete_file_permanently(is_trashed: bool) -> bool {
    is_trashed
}
