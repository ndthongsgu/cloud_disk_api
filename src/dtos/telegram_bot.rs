use serde::{Deserialize, Serialize};
use uuid::Uuid;
use chrono::NaiveDateTime;
use crate::models::telegram_bot::BotStatus;

#[derive(Debug, Deserialize)]
pub struct BotQuery {
    pub page: Option<u32>,
    pub size: Option<u32>,
    pub bot_username: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CreateBotRequest {
    pub code: String,
    pub bot_username: String,
    pub bot_token: String,
    pub chat_id: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateBotRequest {
    pub bot_username: String,
    pub bot_token: Option<String>,
    pub chat_id: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct BotResponse {
    pub id: Uuid,
    pub created_date: NaiveDateTime,
    pub created_by: Option<Uuid>,
    pub updated_date: NaiveDateTime,
    pub updated_by: Option<Uuid>,
    pub version: i32,
    pub code: String,
    pub bot_username: String,
    pub bot_token: String,
    pub chat_id: String,
    pub status: BotStatus,
    #[serde(rename = "statusLabel")]
    pub status_label: String,
    #[serde(rename = "isCanDeActive")]
    pub is_can_deactive: bool,
    #[serde(rename = "isCanActive")]
    pub is_can_active: bool,
    pub total_files_handled: i64,
}
