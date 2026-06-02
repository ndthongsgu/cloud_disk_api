use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Các phân hệ (modules) chính trong hệ thống
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemModule {
    Dashboard,
    Users,
    TelegramBots,
    Folders,
    Files,
    Roles,
}

impl SystemModule {
    pub fn code(&self) -> &'static str {
        match self {
            SystemModule::Dashboard => "dashboard",
            SystemModule::Users => "users",
            SystemModule::TelegramBots => "telegram_bots",
            SystemModule::Folders => "folders",
            SystemModule::Files => "files",
            SystemModule::Roles => "roles",
        }
    }

    pub fn url_path(&self) -> &'static str {
        match self {
            SystemModule::Dashboard => "dashboard",
            SystemModule::Users => "users",
            SystemModule::TelegramBots => "telegram-bots",
            SystemModule::Folders => "folders",
            SystemModule::Files => "files",
            SystemModule::Roles => "roles",
        }
    }
}


/// Các hành động (actions) kiểm tra quyền hạn chuẩn trong hệ thống
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SystemAction {
    View,
    Add,
    Edit,
    Active,
    DeActive,
    All,
    Custom(&'static str),
}

impl SystemAction {
    pub fn field_key(&self) -> &'static str {
        match self {
            SystemAction::View => "view",
            SystemAction::Add => "add",
            SystemAction::Edit => "edit",
            SystemAction::Active => "active",
            SystemAction::DeActive => "deactive",
            SystemAction::All => "all",
            SystemAction::Custom(s) => s,
        }
    }

    pub fn code(&self) -> &'static str {
        match self {
            SystemAction::View => "View",
            SystemAction::Add => "Add",
            SystemAction::Edit => "Edit",
            SystemAction::Active => "Active",
            SystemAction::DeActive => "DeActive",
            SystemAction::All => "All",
            SystemAction::Custom(s) => s,
        }
    }
}


/// Phương thức HTTP dùng cho cấu hình API Path
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum ApiMethod {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
}

impl ApiMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            ApiMethod::GET => "GET",
            ApiMethod::POST => "POST",
            ApiMethod::PUT => "PUT",
            ApiMethod::DELETE => "DELETE",
            ApiMethod::PATCH => "PATCH",
        }
    }
}

/// Định nghĩa đường dẫn API chi tiết gắn liền với quyền
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiPath {
    pub controller_child: String,
    pub method: String,
}

/// Đại diện cho một hành động quyền cụ thể (ví dụ: Xem, Thêm, Sửa, Xóa...)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ActionPermission {
    pub name: String,
    pub code: String,
    pub value: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_apis: Option<Vec<ApiPath>>,
}

/// Đại diện cho một phần tử quyền con (Function/Action level)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PermissionItem {
    pub code: String,
    pub name: String,
    pub controller: String,
    pub path_webs: Vec<String>,
    
    // Các quyền tiêu chuẩn
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub view: Option<ActionPermission>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub add: Option<ActionPermission>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub edit: Option<ActionPermission>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<ActionPermission>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deactive: Option<ActionPermission>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub all: Option<ActionPermission>,

    // Cơ chế phân quyền linh động (Zanzibar / ReBAC / dynamic):
    // flatten HashMap giúp chứa bất kỳ trường động tùy chỉnh nào mà frontend hoặc hệ thống khác gửi lên
    #[serde(flatten)]
    pub extra_actions: HashMap<String, ActionPermission>,
}

/// Đại diện cho một nhóm module (Module level)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ModuleGroup {
    pub id: i32,
    pub name: String,
    pub code: String,
    pub children: Vec<PermissionItem>,
}

/// Toàn bộ cấu trúc phân quyền (RoleGroup) trong hệ thống
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoleGroup {
    pub dashboard: ModuleGroup,
    pub users: ModuleGroup,
    pub telegram_bots: ModuleGroup,
    pub folders: ModuleGroup,
    pub files: ModuleGroup,
    pub roles: ModuleGroup,
}

/// Trả về cấu hình cây phân quyền (RoleGroup) mặc định cho hệ thống Cloud Disk.
/// Cấu hình này chứa các phân hệ chính bao gồm:
/// - dashboard (Thống kê)
/// - users (Người dùng)
/// - telegram_bots (Telegram Nodes)
/// - folders (Thư mục ảo)
/// - files (Tệp tin)
/// - roles (Vai trò & Phân quyền)
pub fn get_default_role_group() -> RoleGroup {
    // Định nghĩa các Action mẫu
    let all_action = ActionPermission {
        name: "Tất cả".to_string(),
        code: SystemAction::All.code().to_string(),
        value: false,
        path_apis: None,
    };

    let view_action = |controller: &str, method: ApiMethod| -> ActionPermission {
        ActionPermission {
            name: "Xem".to_string(),
            code: SystemAction::View.code().to_string(),
            value: false,
            path_apis: Some(vec![ApiPath {
                controller_child: controller.to_string(),
                method: method.as_str().to_string(),
            }]),
        }
    };

    let write_action = |name: &str, code: &str, controller: &str, method: ApiMethod| -> ActionPermission {
        ActionPermission {
            name: name.to_string(),
            code: code.to_string(),
            value: false,
            path_apis: Some(vec![ApiPath {
                controller_child: controller.to_string(),
                method: method.as_str().to_string(),
            }]),
        }
    };

    // 1. Module Dashboard
    let dashboard_group = ModuleGroup {
        id: 1,
        name: "Thống kê".to_string(),
        code: SystemModule::Dashboard.code().to_string(),
        children: vec![
            PermissionItem {
                code: "dashboard_storage".to_string(),
                name: "Thống kê dung lượng".to_string(),
                controller: "dashboard".to_string(),
                path_webs: vec!["dashboard/storage".to_string()],
                view: Some(view_action(&format!("{}/storage", SystemModule::Dashboard.url_path()), ApiMethod::GET)),
                add: None,
                edit: None,
                active: None,
                deactive: None,
                all: Some(all_action.clone()),
                extra_actions: HashMap::new(),
            },
            PermissionItem {
                code: "dashboard_user".to_string(),
                name: "Thống kê người dùng".to_string(),
                controller: "dashboard".to_string(),
                path_webs: vec!["dashboard/user".to_string()],
                view: Some(view_action(&format!("{}/user", SystemModule::Dashboard.url_path()), ApiMethod::GET)),
                add: None,
                edit: None,
                active: None,
                deactive: None,
                all: Some(all_action.clone()),
                extra_actions: HashMap::new(),
            },
        ],
    };

    // 2. Module Users
    let users_group = ModuleGroup {
        id: 2,
        name: "Người dùng".to_string(),
        code: SystemModule::Users.code().to_string(),
        children: vec![PermissionItem {
            code: "users_manage".to_string(),
            name: "Quản lý tài khoản".to_string(),
            controller: "user".to_string(),
            path_webs: vec!["users/list".to_string()],
            view: Some(view_action(SystemModule::Users.url_path(), ApiMethod::GET)),
            add: Some(write_action("Thêm người dùng", SystemAction::Add.code(), SystemModule::Users.url_path(), ApiMethod::POST)),
            edit: Some(write_action("Sửa người dùng", SystemAction::Edit.code(), SystemModule::Users.url_path(), ApiMethod::PUT)),
            active: Some(write_action("Kích hoạt người dùng", SystemAction::Active.code(), SystemModule::Users.url_path(), ApiMethod::POST)),
            deactive: Some(write_action("Khóa người dùng", SystemAction::DeActive.code(), SystemModule::Users.url_path(), ApiMethod::POST)),
            all: Some(all_action.clone()),
            extra_actions: {
                let mut map = HashMap::new();
                // Phân quyền linh động: thêm hành động khóa tài khoản tùy chỉnh
                map.insert(
                    "lockUser".to_string(),
                    ActionPermission {
                        name: "Khóa/Mở tài khoản".to_string(),
                        code: "LockUser".to_string(),
                        value: false,
                        path_apis: Some(vec![ApiPath {
                            controller_child: format!("{}/lock", SystemModule::Users.url_path()),
                            method: ApiMethod::POST.as_str().to_string(),
                        }]),
                    },
                );
                map
            },
        }],
    };

    // 3. Module Telegram Bots (Physical storage nodes)
    let bots_group = ModuleGroup {
        id: 3,
        name: "Telegram Nodes".to_string(),
        code: SystemModule::TelegramBots.code().to_string(),
        children: vec![PermissionItem {
            code: "bots_manage".to_string(),
            name: "Quản lý Bot lưu trữ".to_string(),
            controller: "telegram_bot".to_string(),
            path_webs: vec!["bots/list".to_string()],
            view: Some(view_action(SystemModule::TelegramBots.url_path(), ApiMethod::GET)),
            add: Some(write_action("Thêm Bot", SystemAction::Add.code(), SystemModule::TelegramBots.url_path(), ApiMethod::POST)),
            edit: Some(write_action("Sửa Bot", SystemAction::Edit.code(), SystemModule::TelegramBots.url_path(), ApiMethod::PUT)),
            active: Some(write_action("Kích hoạt Bot", SystemAction::Active.code(), &format!("{}/activate", SystemModule::TelegramBots.url_path()), ApiMethod::POST)),
            deactive: Some(write_action("Ngưng hoạt động Bot", SystemAction::DeActive.code(), &format!("{}/deactivate", SystemModule::TelegramBots.url_path()), ApiMethod::POST)),
            all: Some(all_action.clone()),
            extra_actions: HashMap::new(),
        }],
    };

    // 4. Module Folders
    let folders_group = ModuleGroup {
        id: 4,
        name: "Thư mục ảo".to_string(),
        code: SystemModule::Folders.code().to_string(),
        children: vec![PermissionItem {
            code: "folders_manage".to_string(),
            name: "Quản lý thư mục ảo".to_string(),
            controller: "folder".to_string(),
            path_webs: vec!["folders".to_string()],
            view: Some(view_action(SystemModule::Folders.url_path(), ApiMethod::GET)),
            add: Some(write_action("Tạo thư mục", SystemAction::Add.code(), SystemModule::Folders.url_path(), ApiMethod::POST)),
            edit: Some(write_action("Đổi tên", SystemAction::Edit.code(), SystemModule::Folders.url_path(), ApiMethod::PUT)),
            active: Some(write_action("Kích hoạt thư mục", SystemAction::Active.code(), SystemModule::Folders.url_path(), ApiMethod::POST)),
            deactive: Some(write_action("Ngưng hoạt động thư mục", SystemAction::DeActive.code(), SystemModule::Folders.url_path(), ApiMethod::POST)),
            all: Some(all_action.clone()),
            extra_actions: HashMap::new(),
        }],
    };

    // 5. Module Files
    let files_group = ModuleGroup {
        id: 5,
        name: "Tệp tin".to_string(),
        code: SystemModule::Files.code().to_string(),
        children: vec![PermissionItem {
            code: "files_manage".to_string(),
            name: "Quản lý tệp tin".to_string(),
            controller: "file".to_string(),
            path_webs: vec!["files".to_string()],
            view: Some(view_action(SystemModule::Files.url_path(), ApiMethod::GET)),
            add: Some(write_action("Tải lên", SystemAction::Add.code(), &format!("{}/upload", SystemModule::Files.url_path()), ApiMethod::POST)),
            edit: None,
            active: Some(write_action("Kích hoạt file", SystemAction::Active.code(), SystemModule::Files.url_path(), ApiMethod::POST)),
            deactive: Some(write_action("Ngưng hoạt động file", SystemAction::DeActive.code(), SystemModule::Files.url_path(), ApiMethod::POST)),
            all: Some(all_action.clone()),
            extra_actions: {
                let mut map = HashMap::new();
                map.insert(
                    "download".to_string(),
                    ActionPermission {
                        name: "Tải xuống file".to_string(),
                        code: "Download".to_string(),
                        value: false,
                        path_apis: Some(vec![ApiPath {
                            controller_child: format!("{}/download", SystemModule::Files.url_path()),
                            method: ApiMethod::GET.as_str().to_string(),
                        }]),
                    },
                );
                map.insert(
                    "stream".to_string(),
                    ActionPermission {
                        name: "Xem video streaming".to_string(),
                        code: "Stream".to_string(),
                        value: false,
                        path_apis: Some(vec![ApiPath {
                            controller_child: format!("{}/stream", SystemModule::Files.url_path()),
                            method: ApiMethod::GET.as_str().to_string(),
                        }]),
                    },
                );
                map
            },
        }],
    };

    // 6. Module Roles
    let roles_group = ModuleGroup {
        id: 6,
        name: "Vai trò & Quyền".to_string(),
        code: SystemModule::Roles.code().to_string(),
        children: vec![PermissionItem {
            code: "roles_manage".to_string(),
            name: "Quản lý vai trò".to_string(),
            controller: "role".to_string(),
            path_webs: vec!["roles".to_string()],
            view: Some(view_action(SystemModule::Roles.url_path(), ApiMethod::GET)),
            add: Some(write_action("Thêm vai trò", SystemAction::Add.code(), SystemModule::Roles.url_path(), ApiMethod::POST)),
            edit: Some(write_action("Sửa vai trò", SystemAction::Edit.code(), SystemModule::Roles.url_path(), ApiMethod::PUT)),
            active: Some(write_action("Kích hoạt vai trò", SystemAction::Active.code(), SystemModule::Roles.url_path(), ApiMethod::POST)),
            deactive: Some(write_action("Ngưng hoạt động vai trò", SystemAction::DeActive.code(), SystemModule::Roles.url_path(), ApiMethod::POST)),
            all: Some(all_action.clone()),
            extra_actions: HashMap::new(),
        }],
    };

    RoleGroup {
        dashboard: dashboard_group,
        users: users_group,
        telegram_bots: bots_group,
        folders: folders_group,
        files: files_group,
        roles: roles_group,
    }
}
