use sqlx::{migrate::MigrateDatabase, Sqlite, SqlitePool};

use crate::Result;

const DATABASE_PATH: &str = "./app/database.db";

pub type SqlPool = sqlx::Pool<sqlx::Sqlite>;
pub type SqlConnection = sqlx::pool::PoolConnection<sqlx::Sqlite>;

pub async fn init() -> Result<SqlPool> {
    let does_db_exist = Sqlite::database_exists(DATABASE_PATH)
        .await
        .unwrap_or(false);

    if !does_db_exist {
        debug!("Creating database {DATABASE_PATH}");

        Sqlite::create_database(DATABASE_PATH).await?;
    } else {
        debug!("Database already exists");
    }

    let pool = SqlitePool::connect(DATABASE_PATH).await?;

    match sqlx::migrate!("./migrations").run(&pool).await {
        Ok(_) => debug!("Migration success"),
        Err(error) => panic!("Migration Error: {error}"),
    }

    Ok(pool)
}
