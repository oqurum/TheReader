use crate::Result;

use super::Database;

mod current;

pub async fn start_initiation(database: &Database) -> Result<()> {
    if does_migration_table_exist(database).await? {
        // TODO: Handle Migrations
    } else {
        current::init(database).await?;
    }

    Ok(())
}

async fn does_migration_table_exist(database: &Database) -> Result<bool> {
    let read = database.read().await;

    Ok(read.query_row(
        r#"SELECT EXISTS(SELECT * from sqlite_master WHERE type = "table" AND name = "migration")"#,
        [],
        |v| v.get::<_, bool>(0),
    )?)
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
