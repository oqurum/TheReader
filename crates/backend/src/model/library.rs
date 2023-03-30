use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use crate::{DatabaseAccess, Result};
use common_local::{LibraryId, LibraryType};

use super::{directory::DirectoryModel, AdvRow, TableRow};

pub struct NewLibraryModel {
    pub name: String,
    pub type_of: LibraryType,

    pub is_public: bool,
    pub settings: Option<String>,

    pub scanned_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct LibraryModel {
    pub id: LibraryId,

    pub name: String,
    pub type_of: LibraryType,

    pub is_public: bool,
    pub settings: Option<String>,

    pub scanned_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl TableRow<'_> for LibraryModel {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.next()?,
            name: row.next()?,
            type_of: LibraryType::try_from(row.next::<i32>()?).unwrap(),
            is_public: row.next()?,
            settings: row.next_opt()?,
            scanned_at: row.next()?,
            created_at: row.next()?,
            updated_at: row.next()?,
        })
    }
}

impl NewLibraryModel {
    pub async fn insert(self, db: &dyn DatabaseAccess) -> Result<LibraryModel> {
        let lock = db.write().await;

        lock.execute(
            r#"INSERT INTO library (name, type_of, is_public, settings, scanned_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)"#,
            params![
                &self.name,
                i32::from(self.type_of),
                self.is_public,
                &self.settings,
                self.scanned_at,
                self.created_at,
                self.updated_at,
            ]
        )?;

        Ok(LibraryModel {
            id: LibraryId::from(lock.last_insert_rowid() as usize),
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
    pub async fn delete_by_id(id: LibraryId, db: &dyn DatabaseAccess) -> Result<usize> {
        DirectoryModel::delete_by_library_id(id, db).await?;

        Ok(db
            .write()
            .await
            .execute(r#"DELETE FROM library WHERE id = ?1"#, [id])?)
    }

    pub async fn count(db: &dyn DatabaseAccess) -> Result<usize> {
        Ok(db
            .read()
            .await
            .query_row("SELECT COUNT(*) FROM library", [], |v| v.get(0))?)
    }

    pub async fn get_all(db: &dyn DatabaseAccess) -> Result<Vec<LibraryModel>> {
        let this = db.read().await;

        let mut conn = this.prepare("SELECT * FROM library")?;

        let map = conn.query_map([], |v| LibraryModel::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn find_one_by_name(
        value: &str,
        db: &dyn DatabaseAccess,
    ) -> Result<Option<LibraryModel>> {
        Ok(db
            .read()
            .await
            .query_row(
                r#"SELECT * FROM library WHERE name = ?1"#,
                params![value],
                |v| LibraryModel::from_row(v),
            )
            .optional()?)
    }

    pub async fn find_one_by_id(
        value: LibraryId,
        db: &dyn DatabaseAccess,
    ) -> Result<Option<LibraryModel>> {
        Ok(db
            .read()
            .await
            .query_row(r#"SELECT * FROM library WHERE id = ?1"#, [value], |v| {
                LibraryModel::from_row(v)
            })
            .optional()?)
    }

    pub async fn update(&mut self, db: &dyn DatabaseAccess) -> Result<usize> {
        self.updated_at = Utc::now();

        let write = db.write().await;

        Ok(write.execute(
            "UPDATE library SET name = ?2, updated_at = ?3 WHERE id = ?1",
            params![self.id, &self.name, self.updated_at,],
        )?)
    }
}
