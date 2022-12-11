use chrono::{DateTime, Utc};
use common::{MemberId, ThumbnailStore};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use common_local::{Collection, CollectionId};

use super::{AdvRow, TableRow};
use crate::{DatabaseAccess, Result};

#[derive(Debug)]
pub struct NewCollectionModel {
    pub member_id: MemberId,

    pub name: String,
    pub description: Option<String>,

    pub thumb_url: ThumbnailStore,
}

#[derive(Debug, Serialize)]
pub struct CollectionModel {
    pub id: CollectionId,

    pub member_id: MemberId,

    pub name: String,
    pub description: Option<String>,

    pub thumb_url: ThumbnailStore,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TableRow<'_> for CollectionModel {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.next()?,

            member_id: row.next()?,

            name: row.next()?,
            description: row.next()?,

            thumb_url: ThumbnailStore::from(row.next_opt::<String>()?),

            created_at: row.next()?,
            updated_at: row.next()?,
        })
    }
}

impl From<CollectionModel> for Collection {
    fn from(val: CollectionModel) -> Self {
        Collection {
            id: val.id,
            member_id: val.member_id,
            name: val.name,
            description: val.description,
            thumb_url: val.thumb_url,
            created_at: val.created_at,
            updated_at: val.updated_at,
        }
    }
}

impl NewCollectionModel {
    pub async fn insert(self, db: &dyn DatabaseAccess) -> Result<CollectionModel> {
        let conn = db.write().await;

        let now = Utc::now();

        conn.execute(
            r#"
            INSERT INTO collection (member_id, name, description, thumb_url, created_at, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6)
        "#,
            params![
                self.member_id,
                &self.name,
                &self.description,
                self.thumb_url.as_value(),
                now,
                now,
            ],
        )?;

        Ok(CollectionModel {
            id: CollectionId::from(conn.last_insert_rowid() as usize),
            member_id: self.member_id,
            name: self.name,
            description: self.description,
            thumb_url: self.thumb_url,
            created_at: now,
            updated_at: now,
        })
    }
}

impl CollectionModel {
    pub async fn find_by_member_id(id: MemberId, db: &dyn DatabaseAccess) -> Result<Vec<Self>> {
        let this = db.read().await;

        let mut conn = this.prepare("SELECT * FROM collection WHERE member_id = ?1")?;

        let map = conn.query_map([id], |v| Self::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn find_one_by_id(
        id: CollectionId,
        member_id: MemberId,
        db: &dyn DatabaseAccess,
    ) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(
                r#"SELECT * FROM collection WHERE id = ?1 AND member_id = ?2"#,
                params![id, member_id],
                |v| Self::from_row(v),
            )
            .optional()?)
    }

    pub async fn update(&self, db: &dyn DatabaseAccess) -> Result<()> {
        db.write().await.execute(
            r#"
            UPDATE collection SET
                name = ?2,
                description = ?3,
                thumb_url = ?4,
                updated_at = ?5,
            WHERE id = ?1"#,
            params![
                self.id,
                &self.name,
                &self.description,
                self.thumb_url.as_value(),
                self.updated_at
            ],
        )?;

        Ok(())
    }

    pub async fn delete_by_id(id: CollectionId, db: &dyn DatabaseAccess) -> Result<usize> {
        Ok(db
            .write()
            .await
            .execute("DELETE FROM collection WHERE id = ?1", [id])?)
    }
}
