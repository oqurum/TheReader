use chrono::{NaiveDateTime, Utc};
use serde::Serialize;
use sqlx::{FromRow, SqliteConnection};

use common_local::{LibraryId, LibraryType};

use super::directory::DirectoryModel;
use crate::Result;

pub struct NewLibraryModel {
    pub name: String,
    pub type_of: LibraryType,

    pub is_public: bool,
    pub settings: Option<String>,

    pub scanned_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct LibraryModel {
    pub id: LibraryId,

    pub name: String,
    pub type_of: LibraryType,

    pub is_public: bool,
    pub settings: Option<String>,

    pub scanned_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
}

impl NewLibraryModel {
    pub async fn insert(self, db: &mut SqliteConnection) -> Result<LibraryModel> {
        let res = sqlx::query(
            "INSERT INTO library (name, type_of, is_public, settings, scanned_at, created_at, updated_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&self.name)
        .bind(self.type_of)
        .bind(self.is_public)
        .bind(&self.settings)
        .bind(self.scanned_at)
        .bind(self.created_at)
        .bind(self.updated_at)
        .execute(db).await?;

        Ok(LibraryModel {
            id: LibraryId::from(res.last_insert_rowid()),
            name: self.name,
            type_of: self.type_of,
            is_public: self.is_public,
            settings: self.settings,
            scanned_at: self.scanned_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
        })
    }
}

impl LibraryModel {
    pub async fn delete_by_id(id: LibraryId, db: &mut SqliteConnection) -> Result<u64> {
        DirectoryModel::delete_by_library_id(id, db).await?;

        let res = sqlx::query("DELETE FROM library WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;

        Ok(res.rows_affected())
    }

    pub async fn count(db: &mut SqliteConnection) -> Result<i32> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM library")
            .fetch_one(db)
            .await?)
    }

    pub async fn get_all(db: &mut SqliteConnection) -> Result<Vec<Self>> {
        Ok(sqlx::query_as("SELECT id, name, type_of, is_public, settings, scanned_at, created_at, updated_at FROM library").fetch_all(db).await?)
    }

    pub async fn find_one_by_name(name: &str, db: &mut SqliteConnection) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, name, type_of, is_public, settings, scanned_at, created_at, updated_at FROM library WHERE name = $1"
        ).bind(name).fetch_optional(db).await?)
    }

    pub async fn find_one_by_id(id: LibraryId, db: &mut SqliteConnection) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, name, type_of, is_public, settings, scanned_at, created_at, updated_at FROM library WHERE id = $1",
        ).bind(id).fetch_optional(db).await?)
    }

    pub async fn update(&mut self, db: &mut SqliteConnection) -> Result<u64> {
        self.updated_at = Utc::now().naive_utc();

        let res = sqlx::query("UPDATE library SET name = $2, updated_at = $3 WHERE id = $1")
            .bind(self.id)
            .bind(&self.name)
            .bind(self.updated_at)
            .execute(db)
            .await?;

        Ok(res.rows_affected())
    }
}
