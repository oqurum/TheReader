use chrono::NaiveDateTime;
use common::BookId;

use common_local::{FileId, LibraryId, LibraryType, MediaItem};
use serde::Serialize;
use sqlx::{sqlite::SqliteRow, FromRow, Row, SqliteConnection};

use super::book::BookModel;
use crate::Result;

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

    pub modified_at: NaiveDateTime,
    pub accessed_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
}

#[derive(Debug, Serialize, FromRow)]
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

    pub modified_at: NaiveDateTime,
    pub accessed_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,
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

    pub async fn insert(self, db: &mut SqliteConnection) -> Result<FileModel> {
        let res = sqlx::query(
            r#"INSERT INTO file (path, file_type, file_name, file_size, modified_at, accessed_at, created_at, identifier, hash, library_id, book_id, chapter_count)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)
            "#,
        )
        .bind(&self.path)
        .bind(&self.file_type)
        .bind(&self.file_name)
        .bind(self.file_size)
        .bind(self.modified_at)
        .bind(self.accessed_at)
        .bind(self.created_at)
        .bind(&self.identifier)
        .bind(&self.hash)
        .bind(self.library_id)
        .bind(self.book_id)
        .bind(self.chapter_count)
        .execute(db).await?;

        Ok(self.into_file(FileId::from(res.last_insert_rowid())))
    }
}

impl FileModel {
    pub fn is_file_type_comic(&self) -> bool {
        LibraryType::ComicBook.is_filetype_valid(&self.file_type)
    }

    pub async fn exists(path: &str, hash: &str, db: &mut SqliteConnection) -> Result<bool> {
        Ok(
            sqlx::query("SELECT EXISTS(SELECT id FROM file WHERE path = $1 OR hash = $2)")
                .bind(path)
                .bind(hash)
                .fetch_one(db)
                .await?
                .try_get(0)?,
        )
    }

    pub async fn find_by(
        library: i64,
        offset: i64,
        limit: i64,
        db: &mut SqliteConnection,
    ) -> Result<Vec<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, path, file_name, file_type, file_size, library_id, book_id, chapter_count, identifier, hash, modified_at, accessed_at, created_at, deleted_at FROM file WHERE library_id = $1 LIMIT $2 OFFSET $3"
        ).bind(library).bind(limit).bind(offset).fetch_all(db).await?)
    }

    pub async fn find_with_book_by(
        library: i64,
        offset: i64,
        limit: i64,
        db: &mut SqliteConnection,
    ) -> Result<Vec<FileWithBook>> {
        Ok(sqlx::query_as(
            r#"SELECT * FROM file
                LEFT JOIN book ON book.id = file.book_id
                WHERE file.library_id = $1
                LIMIT $2
                OFFSET $3
            "#,
        )
        .bind(library)
        .bind(limit)
        .bind(offset)
        .fetch_all(db)
        .await?)
    }

    pub async fn find_by_missing_book(db: &mut SqliteConnection) -> Result<Vec<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, path, file_name, file_type, file_size, library_id, book_id, chapter_count, identifier, hash, modified_at, accessed_at, created_at, deleted_at FROM file WHERE book_id = 0 OR book_id = NULL"
        ).fetch_all(db).await?)
    }

    pub async fn find_one_by_hash_or_path(
        path: &str,
        hash: &str,
        db: &mut SqliteConnection,
    ) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, path, file_name, file_type, file_size, library_id, book_id, chapter_count, identifier, hash, modified_at, accessed_at, created_at, deleted_at FROM file WHERE path = $1 OR hash = $2"
        ).bind(path).bind(hash).fetch_optional(db).await?)
    }

    pub async fn find_one_by_id(id: FileId, db: &mut SqliteConnection) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, path, file_name, file_type, file_size, library_id, book_id, chapter_count, identifier, hash, modified_at, accessed_at, created_at, deleted_at FROM file WHERE id = $1"
        ).bind(id).fetch_optional(db).await?)
    }

    pub async fn find_one_by_id_with_book(
        id: FileId,
        db: &mut SqliteConnection,
    ) -> Result<Option<FileWithBook>> {
        Ok(sqlx::query_as(
            "SELECT * FROM file LEFT JOIN book ON book.id = file.book_id WHERE file.id = $1",
        )
        .bind(id)
        .fetch_optional(db)
        .await?)
    }

    pub async fn find_by_book_id(book_id: BookId, db: &mut SqliteConnection) -> Result<Vec<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, path, file_name, file_type, file_size, library_id, book_id, chapter_count, identifier, hash, modified_at, accessed_at, created_at, deleted_at FROM file WHERE book_id = $1"
        ).bind(book_id).fetch_all(db).await?)
    }

    pub async fn find_by_missing_hash(
        offset: i64,
        limit: i64,
        db: &mut SqliteConnection,
    ) -> Result<Vec<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, path, file_name, file_type, file_size, library_id, book_id, chapter_count, identifier, hash, modified_at, accessed_at, created_at, deleted_at FROM file WHERE hash IS NULL LIMIT $1 OFFSET $2"
        ).bind(limit).bind(offset).fetch_all(db).await?)
    }

    pub async fn count_by_missing_hash(db: &mut SqliteConnection) -> Result<i32> {
        Ok(
            sqlx::query_scalar("SELECT COUNT(*) FROM file WHERE hash IS NULL")
                .fetch_one(db)
                .await?,
        )
    }

    pub async fn count(db: &mut SqliteConnection) -> Result<i32> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM file")
            .fetch_one(db)
            .await?)
    }

    pub async fn update_book_id(
        file_id: FileId,
        book_id: BookId,
        db: &mut SqliteConnection,
    ) -> Result<()> {
        sqlx::query("UPDATE file SET book_id = $1 WHERE id = $2")
            .bind(book_id)
            .bind(file_id)
            .execute(db)
            .await?;

        Ok(())
    }

    pub async fn update(&self, db: &mut SqliteConnection) -> Result<()> {
        sqlx::query(
            r#"UPDATE file SET
                path = $2, file_name = $3, file_type = $4, file_size = $5,
                library_id = $6, book_id = $7, chapter_count = $8, identifier = $9,
                modified_at = $10, accessed_at = $11, created_at = $12, deleted_at = $13
            WHERE id = $1"#,
        )
        .bind(self.id)
        .bind(&self.path)
        .bind(&self.file_name)
        .bind(&self.file_type)
        .bind(self.file_size)
        .bind(self.library_id)
        .bind(self.book_id)
        .bind(self.chapter_count)
        .bind(&self.identifier)
        .bind(self.modified_at)
        .bind(self.accessed_at)
        .bind(self.created_at)
        .bind(self.deleted_at)
        .execute(db)
        .await?;

        Ok(())
    }

    pub async fn transfer_book_id(
        old_book_id: BookId,
        new_book_id: BookId,
        db: &mut SqliteConnection,
    ) -> Result<u64> {
        let res = sqlx::query("UPDATE file SET book_id = $1 WHERE book_id = $2")
            .bind(new_book_id)
            .bind(old_book_id)
            .execute(db)
            .await?;

        Ok(res.rows_affected())
    }
}

pub struct FileWithBook {
    pub file: FileModel,
    pub book: Option<BookModel>,
}

impl FromRow<'_, SqliteRow> for FileWithBook {
    fn from_row(row: &SqliteRow) -> sqlx::Result<Self> {
        Ok(Self {
            file: FileModel::from_row(row)?,
            book: if row.len() > 20 {
                Some(BookModel::from_row(row)?)
            } else {
                None
            },
        })
    }
}
