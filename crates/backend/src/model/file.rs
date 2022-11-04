use chrono::{DateTime, Utc};
use common::{util::serialize_datetime_opt, BookId};
use rusqlite::{params, OptionalExtension};

use crate::{DatabaseAccess, Result};
use common_local::{util::serialize_datetime, FileId, LibraryId, MediaItem};
use serde::Serialize;

use super::{book::BookModel, AdvRow, TableRow};

// FileModel

pub struct NewFileModel {
    pub path: String,

    pub file_name: String,
    pub file_type: String,
    pub file_size: i64,

    pub library_id: LibraryId,
    pub book_id: Option<BookId>,
    pub chapter_count: i64,

    pub identifier: Option<String>,
    pub hash: String,

    pub modified_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct FileModel {
    pub id: FileId,

    pub path: String,

    pub file_name: String,
    pub file_type: String,
    pub file_size: i64,

    pub library_id: LibraryId,
    pub book_id: Option<BookId>,
    pub chapter_count: i64,

    pub identifier: Option<String>,
    pub hash: String,

    #[serde(serialize_with = "serialize_datetime")]
    pub modified_at: DateTime<Utc>,
    #[serde(serialize_with = "serialize_datetime")]
    pub accessed_at: DateTime<Utc>,
    #[serde(serialize_with = "serialize_datetime")]
    pub created_at: DateTime<Utc>,
    #[serde(serialize_with = "serialize_datetime_opt")]
    pub deleted_at: Option<DateTime<Utc>>,
}

impl From<FileModel> for MediaItem {
    fn from(file: FileModel) -> Self {
        Self {
            id: file.id,

            path: file.path,

            file_name: file.file_name,
            file_type: file.file_type,
            file_size: file.file_size,

            library_id: file.library_id,
            book_id: file.book_id,
            chapter_count: file.chapter_count as usize,

            identifier: file.identifier,
            hash: file.hash,

            modified_at: file.modified_at.timestamp_millis(),
            accessed_at: file.accessed_at.timestamp_millis(),
            created_at: file.created_at.timestamp_millis(),
            deleted_at: file.deleted_at.map(|v| v.timestamp_millis()),
        }
    }
}

impl TableRow<'_> for FileModel {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.next()?,

            path: row.next()?,

            file_name: row.next()?,
            file_type: row.next()?,
            file_size: row.next()?,

            library_id: row.next()?,
            book_id: row.next()?,
            chapter_count: row.next()?,

            identifier: row.next()?,
            hash: row.next()?,

            modified_at: row.next()?,
            accessed_at: row.next()?,
            created_at: row.next()?,
            deleted_at: row.next_opt()?,
        })
    }
}

impl NewFileModel {
    pub fn into_file(self, id: FileId) -> FileModel {
        FileModel {
            id,
            path: self.path,
            file_name: self.file_name,
            file_type: self.file_type,
            file_size: self.file_size,
            library_id: self.library_id,
            book_id: self.book_id,
            chapter_count: self.chapter_count,
            identifier: self.identifier,
            hash: self.hash,
            modified_at: self.modified_at,
            accessed_at: self.accessed_at,
            created_at: self.created_at,
            deleted_at: self.deleted_at,
        }
    }

    pub async fn insert(self, db: &dyn DatabaseAccess) -> Result<FileModel> {
        let conn = db.write().await;

        conn.execute(r#"
            INSERT INTO file (path, file_type, file_name, file_size, modified_at, accessed_at, created_at, identifier, hash, library_id, book_id, chapter_count)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)
        "#,
        params![
            &self.path, &self.file_type, &self.file_name, self.file_size,
            self.modified_at, self.accessed_at, self.created_at,
            self.identifier.as_deref(), &self.hash,
            self.library_id, self.book_id, self.chapter_count
        ])?;

        Ok(self.into_file(FileId::from(conn.last_insert_rowid() as usize)))
    }
}

impl FileModel {
    pub async fn exists(path: &str, hash: &str, db: &dyn DatabaseAccess) -> Result<bool> {
        Ok(db.read().await.query_row(
            r#"SELECT EXISTS(SELECT id FROM file WHERE path = ?1 OR hash = ?2)"#,
            [path, hash],
            |v| v.get::<_, bool>(0),
        )?)
    }

    pub async fn find_by(
        library: usize,
        offset: usize,
        limit: usize,
        db: &dyn DatabaseAccess,
    ) -> Result<Vec<Self>> {
        let this = db.read().await;

        let mut conn =
            this.prepare("SELECT * FROM file WHERE library_id = ?1 LIMIT ?2 OFFSET ?3")?;

        let map = conn.query_map([library, limit, offset], |v| Self::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn find_with_book_by(
        library: usize,
        offset: usize,
        limit: usize,
        db: &dyn DatabaseAccess,
    ) -> Result<Vec<FileWithBook>> {
        let this = db.read().await;

        let mut conn = this.prepare(
            r#"
            SELECT * FROM file
            LEFT JOIN book ON book.id = file.book_id
            WHERE library_id = ?1
            LIMIT ?2
            OFFSET ?3
        "#,
        )?;

        let map = conn.query_map([library, limit, offset], |v| FileWithBook::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn find_by_missing_book(db: &dyn DatabaseAccess) -> Result<Vec<Self>> {
        let this = db.read().await;

        let mut conn = this.prepare("SELECT * FROM file WHERE book_id = 0 OR book_id = NULL")?;

        let map = conn.query_map([], |v| Self::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn find_one_by_hash_or_path(
        path: &str,
        hash: &str,
        db: &dyn DatabaseAccess,
    ) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(
                r#"SELECT * FROM file WHERE path = ?1 OR hash = ?2"#,
                [path, hash],
                |v| Self::from_row(v),
            )
            .optional()?)
    }

    pub async fn find_one_by_id(id: FileId, db: &dyn DatabaseAccess) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(r#"SELECT * FROM file WHERE id=?1"#, params![id], |v| {
                Self::from_row(v)
            })
            .optional()?)
    }

    pub async fn find_one_by_id_with_book(
        id: FileId,
        db: &dyn DatabaseAccess,
    ) -> Result<Option<FileWithBook>> {
        Ok(db
            .read()
            .await
            .query_row(
                r#"SELECT * FROM file LEFT JOIN book ON book.id = file.book_id WHERE file.id = ?1"#,
                [id],
                |v| FileWithBook::from_row(v),
            )
            .optional()?)
    }

    pub async fn find_by_book_id(book_id: BookId, db: &dyn DatabaseAccess) -> Result<Vec<Self>> {
        let this = db.read().await;

        let mut conn = this.prepare("SELECT * FROM file WHERE book_id=?1")?;

        let map = conn.query_map([book_id], |v| Self::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn find_by_missing_hash(
        offset: usize,
        limit: usize,
        db: &dyn DatabaseAccess,
    ) -> Result<Vec<Self>> {
        let this = db.read().await;

        let mut conn = this.prepare("SELECT * FROM file WHERE hash IS NULL LIMIT ?1 OFFSET ?2")?;

        let map = conn.query_map([limit, offset], |v| Self::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn count_by_missing_hash(db: &dyn DatabaseAccess) -> Result<usize> {
        Ok(db
            .read()
            .await
            .query_row("SELECT COUNT(*) FROM file WHERE hash IS NULL", [], |v| {
                v.get(0)
            })?)
    }

    pub async fn count(db: &dyn DatabaseAccess) -> Result<usize> {
        Ok(db
            .read()
            .await
            .query_row(r#"SELECT COUNT(*) FROM file"#, [], |v| v.get(0))?)
    }

    pub async fn update_book_id(file_id: FileId, book_id: BookId, db: &dyn DatabaseAccess) -> Result<()> {
        db.write().await.execute(
            r#"UPDATE file SET book_id = ?1 WHERE id = ?2"#,
            params![book_id, file_id],
        )?;

        Ok(())
    }

    pub async fn update(&self, db: &dyn DatabaseAccess) -> Result<()> {
        db.write().await.execute(
            r#"
            UPDATE file SET
                path = ?2, file_name = ?3, file_type = ?4, file_size = ?5,
                library_id = ?6, book_id = ?7, chapter_count = ?8, identifier = ?9,
                modified_at = ?10, accessed_at = ?11, created_at = ?12, deleted_at = ?13
            WHERE id = ?1"#,
            params![
                self.id,
                self.path,
                &self.file_name,
                &self.file_type,
                self.file_size,
                self.library_id,
                self.book_id,
                self.chapter_count,
                self.identifier,
                self.modified_at,
                self.accessed_at,
                self.created_at,
                self.deleted_at,
            ],
        )?;

        Ok(())
    }

    pub async fn transfer_book_id(
        old_book_id: BookId,
        new_book_id: BookId,
        db: &dyn DatabaseAccess,
    ) -> Result<usize> {
        Ok(db.write().await.execute(
            r#"UPDATE file SET book_id = ?1 WHERE book_id = ?2"#,
            params![new_book_id, old_book_id],
        )?)
    }
}

pub struct FileWithBook {
    pub file: FileModel,
    pub book: Option<BookModel>,
}

impl TableRow<'_> for FileWithBook {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            file: FileModel::create(row)?,

            book: row
                .has_next()
                .ok()
                .filter(|v| *v)
                .map(|_| BookModel::create(row))
                .transpose()?,
        })
    }
}
