use common::BookId;
use common_local::CollectionId;
use rusqlite::params;

use crate::{DatabaseAccess, Result};
use serde::Serialize;

use super::{AdvRow, TableRow};

#[derive(Debug, Serialize)]
pub struct CollectionItemModel {
    pub collection_id: CollectionId,
    pub book_id: BookId,
}

impl TableRow<'_> for CollectionItemModel {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            collection_id: row.next()?,
            book_id: row.next()?,
        })
    }
}

impl CollectionItemModel {
    pub async fn insert_or_ignore(&self, db: &dyn DatabaseAccess) -> Result<()> {
        db.write().await.execute(
            "INSERT OR IGNORE INTO collection_item (collection_id, book_id) VALUES (?1, ?2)",
            params![self.collection_id, self.book_id],
        )?;

        Ok(())
    }

    pub async fn delete_one(&self, db: &dyn DatabaseAccess) -> Result<()> {
        db.write().await.execute(
            "DELETE FROM collection_item WHERE collection_id = ?1 AND book_id = ?2",
            params![self.collection_id, self.book_id],
        )?;

        Ok(())
    }

    pub async fn find_by_collection_id(
        id: CollectionId,
        db: &dyn DatabaseAccess,
    ) -> Result<Vec<Self>> {
        let this = db.read().await;

        let mut conn = this.prepare("SELECT * FROM collection_item WHERE collection_id = ?1")?;

        let map = conn.query_map([id], |v| Self::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }
}
