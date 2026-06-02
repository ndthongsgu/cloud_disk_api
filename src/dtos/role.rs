use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::NaiveDateTime;

#[derive(Debug, Deserialize)]
pub struct RoleQuery {
    pub page: Option<u32>,
    pub size: Option<u32>,
    pub name: Option<String>,
    pub code: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateRoleRequest {
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: serde_json::Value,
}

#[derive(Debug, Deserialize)]
pub struct UpdateRoleRequest {
    pub name: String,
    pub description: Option<String>,
    pub permissions: serde_json::Value,
}

#[derive(Debug, Serialize)]
pub struct RoleResponse {
    pub id: Uuid,
    pub created_date: NaiveDateTime,
    pub created_by: Option<Uuid>,
    pub updated_date: NaiveDateTime,
    pub updated_by: Option<Uuid>,
    pub version: i32,
    pub code: String,
    pub name: String,
    pub description: Option<String>,
    pub permissions: serde_json::Value,
}
