use crate::Result;
use sqlx::{SqlitePool, Sqlite, migrate::MigrateDatabase};

const DATABASE_PATH: &str = "./app/database.db";

pub type SqlPool = sqlx::Pool<sqlx::Sqlite>;
pub type SqlConnection = sqlx::pool::PoolConnection<sqlx::Sqlite>;

mod migrations;


pub async fn init() -> Result<SqlPool> {
    if !Sqlite::database_exists(DATABASE_PATH).await.unwrap_or(false) {
        tracing::debug!("Creating database {DATABASE_PATH}");

        match Sqlite::create_database(DATABASE_PATH).await {
            Ok(_) => tracing::debug!("Create db success"),
            Err(error) => tracing::error!("error: {error}"),
        }
    } else {
        tracing::debug!("Database already exists");
    }

    let pool = SqlitePool::connect(DATABASE_PATH).await?;

    migrations::start_initiation(&pool).await?;

    Ok(pool)
}
