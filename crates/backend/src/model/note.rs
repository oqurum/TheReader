use chrono::{DateTime, Utc};
use common::MemberId;
use rusqlite::{params, OptionalExtension};

use crate::{DatabaseAccess, Result};
use common_local::{util::serialize_datetime, FileId};
use serde::Serialize;

use super::{AdvRow, TableRow};

#[derive(Debug, Serialize)]
pub struct FileNoteModel {
    pub file_id: FileId,
    pub member_id: MemberId,

    pub data: String,
    pub data_size: i64,

    #[serde(serialize_with = "serialize_datetime")]
    pub updated_at: DateTime<Utc>,
    #[serde(serialize_with = "serialize_datetime")]
    pub created_at: DateTime<Utc>,
}

impl FileNoteModel {
    pub fn new(file_id: FileId, member_id: MemberId, data: String) -> Self {
        Self {
            file_id,
            member_id,
            data_size: data.len() as i64,
            data,
            updated_at: Utc::now(),
            created_at: Utc::now(),
        }
    }
}

impl TableRow<'_> for FileNoteModel {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            file_id: row.next()?,
            member_id: row.next()?,

            data: row.next()?,

            data_size: row.next()?,

            updated_at: row.next()?,
            created_at: row.next()?,
        })
    }
}

impl FileNoteModel {
    pub async fn insert_or_update(&self, db: &dyn DatabaseAccess) -> Result<()> {
        if Self::find_one(self.file_id, self.member_id, db)
            .await?
            .is_some()
        {
            db.write().await.execute(
                r#"UPDATE file_note SET data = ?1, data_size = ?2, updated_at = ?3 WHERE file_id = ?4 AND user_id = ?5"#,
                params![self.data, self.data_size, self.updated_at, self.file_id, self.member_id]
            )?;
        } else {
            db.write().await.execute(
                r#"INSERT INTO file_note (file_id, user_id, data, data_size, updated_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
                params![self.file_id, self.member_id, self.data, self.data_size, self.updated_at, self.created_at]
            )?;
        }

        Ok(())
    }

    pub async fn find_one(
        file_id: FileId,
        member_id: MemberId,
        db: &dyn DatabaseAccess,
    ) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(
                "SELECT * FROM file_note WHERE user_id = ?1 AND file_id = ?2",
                params![member_id, file_id],
                |v| Self::from_row(v),
            )
            .optional()?)
    }

    pub async fn delete_one(file_id: FileId, member_id: MemberId, db: &dyn DatabaseAccess) -> Result<()> {
        db.write().await.execute(
            "DELETE FROM file_note WHERE user_id = ?1 AND file_id = ?2",
            params![member_id, file_id],
        )?;

        Ok(())
    }
}
