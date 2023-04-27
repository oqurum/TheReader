use common_local::LibraryId;
use sqlx::{FromRow, SqliteConnection};

use crate::Result;

#[derive(FromRow)]
pub struct DirectoryModel {
    pub library_id: LibraryId,
    pub path: String,
}

impl DirectoryModel {
    pub async fn insert(&self, db: &mut SqliteConnection) -> Result<()> {
        sqlx::query("INSERT INTO directory (library_id, path) VALUES ($1, $2)")
            .bind(self.library_id)
            .bind(&self.path)
            .execute(db)
            .await?;

        Ok(())
    }

    pub async fn remove_by_path(path: &str, db: &mut SqliteConnection) -> Result<u64> {
        let res = sqlx::query("DELETE FROM directory WHERE path = $1")
            .bind(path)
            .execute(db)
            .await?;

        Ok(res.rows_affected())
    }

    pub async fn delete_by_library_id(id: LibraryId, db: &mut SqliteConnection) -> Result<u64> {
        let res = sqlx::query("DELETE FROM directory WHERE library_id = $1")
            .bind(id)
            .execute(db)
            .await?;

        Ok(res.rows_affected())
    }

    pub async fn find_directories_by_library_id(
        library_id: LibraryId,
        db: &mut SqliteConnection,
    ) -> Result<Vec<DirectoryModel>> {
        Ok(
            sqlx::query_as("SELECT * FROM directory WHERE library_id = $1")
                .bind(library_id)
                .fetch_all(db)
                .await?,
        )
    }

    pub async fn get_all(db: &mut SqliteConnection) -> Result<Vec<DirectoryModel>> {
        Ok(sqlx::query_as("SELECT * FROM directory")
            .fetch_all(db)
            .await?)
    }
}
