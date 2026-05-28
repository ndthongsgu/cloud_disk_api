mod common;
mod handlers;
mod middlewares;
mod models;
mod routes;
mod services;
mod utils;


use actix_web::{App, HttpServer};
use dotenvy::dotenv;
use services::db::init_db_pool;
use std::env;
use tracing_actix_web::TracingLogger;
use utils::{config::database_url, logger::init_logger};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    dotenv().ok();
    init_logger();

    let database_url = database_url();
    let db_pool = init_db_pool(&database_url)
        .await
        .expect("Failed to connect to PostgreSQL");

    let port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    tracing::info!(port = %port, "Server running at http://127.0.0.1:{}", port);

    HttpServer::new(move || {
        App::new()
            .wrap(TracingLogger::default())
            .app_data(db_pool.clone())
            .configure(routes::configure_routes)
    })
    .bind(format!("127.0.0.1:{}", port))?
    .run()
    .await
}