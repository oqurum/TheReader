use chrono::{DateTime, Utc};
use common::{BookId, MemberId};
use rusqlite::{params, OptionalExtension};

use crate::{DatabaseAccess, Result};
use common_local::{FileId, Progression};
use serde::Serialize;

use super::{book::BookModel, AdvRow, TableRow};

#[derive(Debug, Serialize)]
pub struct FileProgressionModel {
    pub book_id: BookId,
    pub file_id: FileId,
    pub user_id: MemberId,

    pub type_of: u8,

    // Ebook/Audiobook
    pub chapter: Option<i64>,

    // Ebook
    pub page: Option<i64>, // TODO: Remove page. Change to byte pos. Most accurate since screen sizes can change.
    pub char_pos: Option<i64>,

    // Audiobook
    pub seek_pos: Option<i64>,

    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl FileProgressionModel {
    pub fn new(progress: Progression, user_id: MemberId, book_id: BookId, file_id: FileId) -> Self {
        match progress {
            Progression::Complete => Self {
                book_id,
                file_id,
                user_id,
                type_of: 0,
                chapter: None,
                page: None,
                char_pos: None,
                seek_pos: None,
                updated_at: Utc::now(),
                created_at: Utc::now(),
            },

            Progression::Ebook {
                chapter,
                page,
                char_pos,
            } => Self {
                book_id,
                file_id,
                user_id,
                type_of: 1,
                char_pos: Some(char_pos),
                chapter: Some(chapter),
                page: Some(page),
                seek_pos: None,
                updated_at: Utc::now(),
                created_at: Utc::now(),
            },

            Progression::AudioBook { chapter, seek_pos } => Self {
                book_id,
                file_id,
                user_id,
                type_of: 2,
                chapter: Some(chapter),
                page: None,
                char_pos: None,
                seek_pos: Some(seek_pos),
                updated_at: Utc::now(),
                created_at: Utc::now(),
            },
        }
    }
}

impl TableRow<'_> for FileProgressionModel {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            book_id: row.next()?,
            file_id: row.next()?,
            user_id: row.next()?,

            type_of: row.next()?,

            chapter: row.next()?,

            page: row.next()?,
            char_pos: row.next()?,

            seek_pos: row.next()?,

            updated_at: row.next()?,
            created_at: row.next()?,
        })
    }
}

impl From<FileProgressionModel> for Progression {
    fn from(val: FileProgressionModel) -> Self {
        match val.type_of {
            0 => Progression::Complete,

            1 => Progression::Ebook {
                char_pos: val.char_pos.unwrap(),
                chapter: val.chapter.unwrap(),
                page: val.page.unwrap(),
            },

            2 => Progression::AudioBook {
                chapter: val.chapter.unwrap(),
                seek_pos: val.seek_pos.unwrap(),
            },

            _ => unreachable!(),
        }
    }
}

impl FileProgressionModel {
    pub async fn insert_or_update(
        member_id: MemberId,
        book_id: BookId,
        file_id: FileId,
        progress: Progression,
        db: &dyn DatabaseAccess,
    ) -> Result<()> {
        let prog = Self::new(progress, member_id, book_id, file_id);

        if Self::find_one(member_id, file_id, db).await?.is_some() {
            db.write().await.execute(
                r#"UPDATE file_progression SET chapter = ?1, char_pos = ?2, page = ?3, seek_pos = ?4, updated_at = ?5 WHERE book_id = ?6 AND file_id = ?7 AND user_id = ?8"#,
                params![prog.chapter, prog.char_pos, prog.page, prog.seek_pos, prog.updated_at, prog.book_id, prog.file_id, prog.user_id]
            )?;
        } else {
            db.write().await.execute(
                r#"INSERT INTO file_progression (book_id, file_id, user_id, type_of, chapter, char_pos, page, seek_pos, updated_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)"#,
                params![prog.book_id, prog.file_id, prog.user_id, prog.type_of, prog.chapter, prog.char_pos, prog.page, prog.seek_pos, prog.updated_at, prog.created_at]
            )?;
        }

        Ok(())
    }

    pub async fn find_one(
        member_id: MemberId,
        file_id: FileId,
        db: &dyn DatabaseAccess,
    ) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(
                "SELECT * FROM file_progression WHERE user_id = ?1 AND file_id = ?2",
                params![member_id, file_id],
                |v| Self::from_row(v),
            )
            .optional()?)
    }

    pub async fn delete_one(
        member_id: MemberId,
        file_id: FileId,
        db: &dyn DatabaseAccess,
    ) -> Result<()> {
        db.write().await.execute(
            "DELETE FROM file_progression WHERE user_id = ?1 AND file_id = ?2",
            params![member_id, file_id],
        )?;

        Ok(())
    }

    pub async fn find_one_by_book_id(
        member_id: MemberId,
        book_id: BookId,
        db: &dyn DatabaseAccess,
    ) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(
                "SELECT * FROM file_progression WHERE user_id = ?1 AND book_id = ?2",
                params![member_id, book_id],
                |v| Self::from_row(v),
            )
            .optional()?)
    }

    pub async fn get_member_progression_and_books(
        member_id: MemberId,
        offset: usize,
        limit: usize,
        db: &dyn DatabaseAccess,
    ) -> Result<Vec<(Self, BookModel)>> {
        let read = db.read().await;

        let mut statement = read.prepare(
            r"
            SELECT * FROM file_progression
            JOIN book ON book.id = file_progression.book_id
            WHERE user_id = ?1 AND type_of = ?2
            ORDER BY updated_at DESC
            LIMIT ?3 OFFSET ?4",
        )?;

        let rows = statement.query_map(params![member_id, 1, limit, offset], |v| {
            let mut row = AdvRow::from(v);

            Ok((Self::create(&mut row)?, BookModel::create(&mut row)?))
        })?;

        Ok(rows.collect::<rusqlite::Result<_>>()?)
    }
}
