use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::NaiveDateTime;

#[derive(Debug, Deserialize)]
pub struct UserSearchRequest {
    pub page: Option<u32>,
    pub size: Option<u32>,
    pub username: Option<String>,
    pub full_name: Option<String>,
    pub email: Option<String>,
    pub phone: Option<String>,
    pub department_name: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub department_name: Option<String>,
    pub storage_quota_bytes: Option<i64>,
    pub roles: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub department_name: Option<String>,
    pub storage_quota_bytes: Option<i64>,
    pub roles: Vec<Uuid>,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub created_date: NaiveDateTime,
    pub created_by: Option<Uuid>,
    pub updated_date: NaiveDateTime,
    pub updated_by: Option<Uuid>,
    pub version: i32,
    pub is_admin: bool,
    pub username: String,
    pub status: String,
    #[serde(rename = "statusLabel")]
    pub status_label: String,
    #[serde(rename = "isCanDeActive")]
    pub is_can_deactive: bool,
    #[serde(rename = "isCanActive")]
    pub is_can_active: bool,
    pub code: String,
    pub full_name: String,
    pub email: String,
    pub phone: Option<String>,
    pub department_name: Option<String>,
    pub storage_quota_bytes: Option<i64>,
    pub used_storage_bytes: i64,
    pub roles: Vec<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct UserSelectRequest {
    pub q: Option<String>,
    pub limit: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct UserSelectResponse {
    pub id: Uuid,
    pub username: String,
    pub full_name: String,
    pub code: String,
}
