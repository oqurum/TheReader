use common::BookId;
use common_local::CollectionId;
use sqlx::{FromRow, SqliteConnection};

use crate::Result;
use serde::Serialize;

#[derive(Debug, Serialize, FromRow)]
pub struct CollectionItemModel {
    pub collection_id: CollectionId,
    pub book_id: BookId,
}

impl CollectionItemModel {
    pub async fn insert_or_ignore(&self, db: &mut SqliteConnection) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO collection_item (collection_id, book_id) VALUES ($1, $2)",
        )
        .bind(self.collection_id)
        .bind(self.book_id)
        .execute(db)
        .await?;

        Ok(())
    }

    pub async fn delete_one(&self, db: &mut SqliteConnection) -> Result<()> {
        sqlx::query("DELETE FROM collection_item WHERE collection_id = $1 AND book_id = $2")
            .bind(self.collection_id)
            .bind(self.book_id)
            .execute(db)
            .await?;

        Ok(())
    }

    pub async fn find_by_collection_id(
        id: CollectionId,
        db: &mut SqliteConnection,
    ) -> Result<Vec<Self>> {
        Ok(
            sqlx::query_as("SELECT * FROM collection_item WHERE collection_id = $1")
                .bind(id)
                .fetch_all(db)
                .await?,
        )
    }
}
