use std::env;

pub fn database_url() -> String {
    env::var("DATABASE_URL").expect("DATABASE_URL must be set")
}
