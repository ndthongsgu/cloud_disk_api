use std::time::Instant;

use actix_web::{get, web, HttpRequest, HttpResponse, Result};
use serde::Deserialize;
use sqlx::{PgPool, query_builder::QueryBuilder};

use crate::models::user::User;

#[derive(Debug, Deserialize)]
pub struct UserQuery {
    pub page: Option<u32>,
    pub size: Option<u32>,
    pub username: Option<String>,
}

#[get("/")]
pub async fn hello() -> HttpResponse {
    HttpResponse::Ok().body("Hello Actix-Web!")
}

#[get("/users")]
pub async fn get_users(
    db: web::Data<PgPool>,
    query: web::Query<UserQuery>,
    req: HttpRequest,
) -> Result<HttpResponse> {
    let query = query.into_inner();
    let limit = query.size.unwrap_or(100).clamp(1, 100);
    let offset = query.page.unwrap_or(1).saturating_sub(1) * limit;

    tracing::info!(
        method = %req.method(),
        path = %req.path(),
        uri = %req.uri(),
        query = ?query,
        "Incoming /users request"
    );

    let start = Instant::now();

    let mut builder = QueryBuilder::new("SELECT id, created_date, created_by, updated_date, updated_by, version, is_deleted, username, password_hash, status, code, full_name, email, phone, department_name, storage_quota_bytes, used_storage_bytes FROM users");

    if let Some(username) = &query.username {
        builder
            .push(" WHERE username ILIKE ")
            .push_bind(format!("%{}%", username))
            .push(" ORDER BY id LIMIT ")
            .push_bind(limit as i64)
            .push(" OFFSET ")
            .push_bind(offset as i64);
    } else {
        builder
            .push(" ORDER BY id LIMIT ")
            .push_bind(limit as i64)
            .push(" OFFSET ")
            .push_bind(offset as i64);
    }

    let result = builder
        .build_query_as::<User>()
        .fetch_all(db.get_ref())
        .await;

    let elapsed_ms = start.elapsed().as_millis();

    match result {
        Ok(users) => {
            tracing::debug!(count = users.len(), duration_ms = elapsed_ms, "Fetched users successfully");
            Ok(HttpResponse::Ok().json(users))
        }
        Err(error) => {
            tracing::error!(error = %error, duration_ms = elapsed_ms, query = ?query, "Failed to fetch users");
            Ok(HttpResponse::InternalServerError().body("Internal server error"))
        }
    }
}
