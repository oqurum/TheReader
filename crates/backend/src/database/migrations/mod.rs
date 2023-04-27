use sqlx::{Executor, Row};

use crate::Result;

use super::SqlPool;

mod current;

pub async fn start_initiation(database: &SqlPool) -> Result<()> {
    if does_migration_table_exist(database).await? {
        // TODO: Handle Migrations
    } else {
        current::init(database).await?;
    }

    Ok(())
}

async fn does_migration_table_exist(database: &SqlPool) -> Result<bool> {
    let mut read = database.acquire().await?;

    Ok(sqlx::query(r#"SELECT EXISTS(SELECT * from sqlite_master WHERE type = "table" AND name = "migration")"#).fetch_one(&mut *read).await?.try_get(0)?)
}

// struct MigrationModel {
//     id: i32,
//
//     duration: i32,
//
//     name: String,
//     notes: String,
//
//     created_at: DateTime<Utc>,
// }
