use chrono::{Utc, DateTime, TimeZone};
use common::BookId;
use rusqlite::{params, OptionalExtension};

use common_local::{LibraryId, util::serialize_datetime, FileId, MediaItem};
use serde::Serialize;
use crate::{Result, database::Database};

use super::{TableRow, AdvRow, book::BookModel};




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
    pub hash: Option<String>,

    pub modified_at: DateTime<Utc>,
    pub accessed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
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
    pub hash: Option<String>,

    #[serde(serialize_with = "serialize_datetime")]
    pub modified_at: DateTime<Utc>,
    #[serde(serialize_with = "serialize_datetime")]
    pub accessed_at: DateTime<Utc>,
    #[serde(serialize_with = "serialize_datetime")]
    pub created_at: DateTime<Utc>,
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

            modified_at: file.modified_at.timestamp_millis(),
            accessed_at: file.accessed_at.timestamp_millis(),
            created_at: file.created_at.timestamp_millis(),
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

            modified_at: Utc.timestamp_millis(row.next()?),
            accessed_at: Utc.timestamp_millis(row.next()?),
            created_at: Utc.timestamp_millis(row.next()?),
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
        }
    }

    pub async fn insert(self, db: &Database) -> Result<FileModel> {
        let conn = db.write().await;

        conn.execute(r#"
            INSERT INTO file (path, file_type, file_name, file_size, modified_at, accessed_at, created_at, identifier, hash, library_id, book_id, chapter_count)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
        "#,
        params![
            &self.path, &self.file_type, &self.file_name, self.file_size,
            self.modified_at.timestamp_millis(), self.accessed_at.timestamp_millis(), self.created_at.timestamp_millis(),
            self.identifier.as_deref(), self.hash.as_deref(),
            self.library_id, self.book_id, self.chapter_count
        ])?;

        Ok(self.into_file(FileId::from(conn.last_insert_rowid() as usize)))
    }
}


impl FileModel {
    pub async fn path_exists(path: &str, db: &Database) -> Result<bool> {
        Ok(db.read().await.query_row(r#"SELECT id FROM file WHERE path = ?1"#, [path], |_| Ok(1)).optional()?.is_some())
    }

    pub async fn find_by(library: usize, offset: usize, limit: usize, db: &Database) -> Result<Vec<Self>> {
        let this = db.read().await;

        let mut conn = this.prepare("SELECT * FROM file WHERE library_id = ?1 LIMIT ?2 OFFSET ?3")?;

        let map = conn.query_map([library, limit, offset], |v| Self::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn find_with_book_by(library: usize, offset: usize, limit: usize, db: &Database) -> Result<Vec<FileWithBook>> {
        let this = db.read().await;

        let mut conn = this.prepare(r#"
            SELECT * FROM file
            LEFT JOIN book ON book.id = file.book_id
            WHERE library_id = ?1
            LIMIT ?2
            OFFSET ?3
        "#)?;

        let map = conn.query_map([library, limit, offset], |v| FileWithBook::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn find_by_missing_book(db: &Database) -> Result<Vec<Self>> {
        let this = db.read().await;

        let mut conn = this.prepare("SELECT * FROM file WHERE book_id = 0 OR book_id = NULL")?;

        let map = conn.query_map([], |v| Self::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn find_one_by_id(id: FileId, db: &Database) -> Result<Option<Self>> {
        Ok(db.read().await.query_row(
            r#"SELECT * FROM file WHERE id=?1"#,
            params![id],
            |v| Self::from_row(v)
        ).optional()?)
    }

    pub async fn find_one_by_id_with_book(id: FileId, db: &Database) -> Result<Option<FileWithBook>> {
        Ok(db.read().await.query_row(
            r#"SELECT * FROM file LEFT JOIN book ON book.id = file.book_id WHERE file.id = ?1"#,
            [id],
            |v| FileWithBook::from_row(v)
        ).optional()?)
    }

    pub async fn find_by_book_id(book_id: BookId, db: &Database) -> Result<Vec<Self>> {
        let this = db.read().await;

        let mut conn = this.prepare("SELECT * FROM file WHERE book_id=?1")?;

        let map = conn.query_map([book_id], |v| Self::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn find_by_missing_hash(offset: usize, limit: usize, db: &Database) -> Result<Vec<Self>> {
        let this = db.read().await;

        let mut conn = this.prepare("SELECT * FROM file WHERE hash IS NULL LIMIT ?1 OFFSET ?2")?;

        let map = conn.query_map([ limit, offset ], |v| Self::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn count_by_missing_hash(db: &Database) -> Result<usize> {
        Ok(db.read().await.query_row("SELECT COUNT(*) FROM file WHERE hash IS NULL", [], |v| v.get(0))?)
    }

    pub async fn count(db: &Database) -> Result<usize> {
        Ok(db.read().await.query_row(r#"SELECT COUNT(*) FROM file"#, [], |v| v.get(0))?)
    }

    pub async fn update_book_id(file_id: FileId, book_id: BookId, db: &Database) -> Result<()> {
        db.write().await
        .execute(r#"UPDATE file SET book_id = ?1 WHERE id = ?2"#,
            params![book_id, file_id]
        )?;

        Ok(())
    }

    pub async fn transfer_book_id(old_book_id: BookId, new_book_id: BookId, db: &Database) -> Result<usize> {
        Ok(db.write().await
        .execute(r#"UPDATE file SET book_id = ?1 WHERE book_id = ?2"#,
            params![new_book_id, old_book_id]
        )?)
    }
}




pub struct FileWithBook {
    pub file: FileModel,
    pub book: Option<BookModel>
}

impl TableRow<'_> for FileWithBook {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            file: FileModel::create(row)?,

            book: row.has_next()
                .ok().filter(|v| *v)
                .map(|_| BookModel::create(row))
                .transpose()?
        })
    }
}
