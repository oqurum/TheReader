use chrono::{DateTime, Utc, NaiveDateTime};
use common::{MemberId, ThumbnailStore};
use serde::Serialize;

use common_local::{Collection, CollectionId};
use sqlx::{FromRow, SqliteConnection};

use crate::Result;

#[derive(Debug)]
pub struct NewCollectionModel {
    pub member_id: MemberId,

    pub name: String,
    pub description: Option<String>,

    pub thumb_url: ThumbnailStore,
}

#[derive(Debug, Serialize, FromRow)]
pub struct CollectionModel {
    pub id: CollectionId,

    pub member_id: MemberId,

    pub name: String,
    pub description: Option<String>,

    pub thumb_url: ThumbnailStore,

    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
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
    pub async fn insert(self, db: &mut SqliteConnection) -> Result<CollectionModel> {
        let now = Utc::now().naive_utc();

        let thumb_url = self.thumb_url.as_value();

        let res = sqlx::query(
            "INSERT INTO collection (member_id, name, description, thumb_url, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $5)"
        ).bind(self.member_id).bind(&self.name).bind(&self.description).bind(self.thumb_url.as_value()).bind(now).execute(db).await?;

        Ok(CollectionModel {
            id: CollectionId::from(res.last_insert_rowid()),
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
    pub async fn count_by_member_id(id: MemberId, db: &mut SqliteConnection) -> Result<i32> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM collection WHERE member_id = $1").bind(id).fetch_one(db).await?)
    }

    pub async fn find_by_member_id(id: MemberId, db: &mut SqliteConnection) -> Result<Vec<Self>> {
        Ok(sqlx::query_as("SELECT * FROM collection WHERE member_id = $1").bind(id).fetch_all(db).await?)
    }

    pub async fn find_one_by_id(
        id: CollectionId,
        member_id: MemberId,
        db: &mut SqliteConnection,
    ) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT * FROM collection WHERE id = $1 AND member_id = $2"
        ).bind(id).bind(member_id).fetch_optional(db).await?)
    }

    pub async fn update(&mut self, db: &mut SqliteConnection) -> Result<()> {
        self.updated_at = Utc::now().naive_utc();

        sqlx::query(
            r#"UPDATE collection SET
                name = $2,
                description = $3,
                thumb_url = $4,
                updated_at = $5
            WHERE id = $1"#,
        ).bind(self.id).bind(&self.name).bind(&self.description).bind(&self.thumb_url).bind(self.updated_at).execute(db).await?;

        Ok(())
    }

    pub async fn delete_by_id(id: CollectionId, db: &mut SqliteConnection) -> Result<u64> {
        let res = sqlx::query(
            "DELETE FROM collection WHERE id = $1"
        ).bind(id).execute(db).await?;

        Ok(res.rows_affected())
    }
}
