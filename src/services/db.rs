use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn init_db_pool(database_url: &str) -> Result<PgPool, sqlx::Error> {
    PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await
}
