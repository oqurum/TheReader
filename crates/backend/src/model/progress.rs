use chrono::{Utc, NaiveDateTime};
use common::{BookId, MemberId};
use sqlx::{FromRow, SqliteConnection, Row};

use crate::Result;
use common_local::{FileId, Progression};
use serde::Serialize;

use super::book::BookModel;

#[derive(Debug, Serialize, FromRow)]
pub struct FileProgressionModel {
    pub book_id: BookId,
    pub file_id: FileId,
    pub user_id: MemberId,

    pub type_of: i64, // TODO: Supposed to be u8

    // Ebook/Audiobook
    pub chapter: Option<i64>,

    // Ebook
    pub page: Option<i64>, // TODO: Remove page. Change to byte pos. Most accurate since screen sizes can change.
    pub char_pos: Option<i64>,

    // Audiobook
    pub seek_pos: Option<i64>,

    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
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
                updated_at: Utc::now().naive_utc(),
                created_at: Utc::now().naive_utc(),
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
                updated_at: Utc::now().naive_utc(),
                created_at: Utc::now().naive_utc(),
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
                updated_at: Utc::now().naive_utc(),
                created_at: Utc::now().naive_utc(),
            },
        }
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
        db: &mut SqliteConnection,
    ) -> Result<()> {
        let prog = Self::new(progress, member_id, book_id, file_id);

        if Self::find_one(member_id, file_id, db).await?.is_some() {
            sqlx::query(
                "UPDATE file_progression SET chapter = $1, char_pos = $2, page = $3, seek_pos = $4, updated_at = $5 WHERE book_id = $6 AND file_id = $7 AND user_id = $8",
            )
            .bind(prog.chapter).bind(prog.char_pos).bind(prog.page).bind(prog.seek_pos).bind(prog.updated_at)
            .bind(prog.book_id).bind(prog.file_id).bind(prog.user_id)
            .execute(db).await?;
        } else {
            sqlx::query(
                "INSERT INTO file_progression (book_id, file_id, user_id, type_of, chapter, char_pos, page, seek_pos, updated_at, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)",
            )
            .bind(prog.book_id).bind(prog.file_id).bind(prog.user_id).bind(prog.type_of).bind(prog.chapter).bind(prog.char_pos)
            .bind(prog.page).bind(prog.seek_pos).bind(prog.updated_at).bind(prog.created_at)
            .execute(db).await?;
        }

        Ok(())
    }

    pub async fn find_one(
        member_id: MemberId,
        file_id: FileId,
        db: &mut SqliteConnection,
    ) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT book_id, file_id, user_id, type_of, chapter, page, char_pos, seek_pos, updated_at, created_at FROM file_progression WHERE user_id = $1 AND file_id = $2"
        ).bind(member_id).bind(file_id).fetch_optional(db).await?)
    }

    pub async fn delete_one(
        member_id: MemberId,
        file_id: FileId,
        db: &mut SqliteConnection,
    ) -> Result<()> {
        sqlx::query(
            "DELETE FROM file_progression WHERE user_id = $1 AND file_id = $2"
        ).bind(member_id).bind(file_id).execute(db).await?;

        Ok(())
    }

    pub async fn find_one_by_book_id(
        member_id: MemberId,
        book_id: BookId,
        db: &mut SqliteConnection,
    ) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT book_id, file_id, user_id, type_of, chapter, page, char_pos, seek_pos, updated_at, created_at FROM file_progression WHERE user_id = $1 AND book_id = $2"
        ).bind(member_id).bind(book_id).fetch_optional(db).await?)
    }

    pub async fn get_member_progression_and_books(
        member_id: MemberId,
        offset: i64,
        limit: i64,
        db: &mut SqliteConnection,
    ) -> Result<Vec<(Self, BookModel)>> {
        let items = sqlx::query(
            r#"SELECT file_progression.book_id, file_progression.file_id, file_progression.user_id, file_progression.type_of, file_progression.chapter, file_progression.page, file_progression.char_pos, file_progression.seek_pos, file_progression.updated_at, file_progression.created_at,
                book.id, book.library_id, book.type_of, book.parent_id, book.source, book.file_item_count, book.title, book.original_title, book.description, book.rating, book.thumb_url, book.cached, book."index", book.refreshed_at, book.created_at, book.updated_at, book.deleted_at, book.available_at, book.year
            FROM file_progression
                JOIN book ON book.id = file_progression.book_id
            WHERE file_progression.user_id = $1 AND file_progression.type_of = $2
            ORDER BY file_progression.updated_at DESC
            LIMIT $3 OFFSET $4"#,
        )
        .bind(member_id)
        .bind(1)
        .bind(limit)
        .bind(offset)
        .fetch_all(db).await?;

        // TODO: Optimize. Don't need to use fetch_all
        items.into_iter().map(|v| {
            let prog = Self {
                book_id: v.try_get(0)?,
                file_id: v.try_get(1)?,
                user_id: v.try_get(2)?,
                type_of: v.try_get(3)?,
                chapter: v.try_get(4)?,
                page: v.try_get(5)?,
                char_pos: v.try_get(6)?,
                seek_pos: v.try_get(7)?,
                updated_at: v.try_get(8)?,
                created_at: v.try_get(9)?,
            };

            let book = BookModel {
                id: v.try_get(10)?,
                library_id: v.try_get(11)?,
                type_of: v.try_get(12)?,
                parent_id: v.try_get(13)?,
                source: v.try_get(14)?,
                file_item_count: v.try_get(15)?,
                title: v.try_get(16)?,
                original_title: v.try_get(17)?,
                description: v.try_get(18)?,
                rating: v.try_get(19)?,
                thumb_url: v.try_get(20)?,
                cached: v.try_get(21)?,
                index: v.try_get(22)?,
                refreshed_at: v.try_get(23)?,
                created_at: v.try_get(24)?,
                updated_at: v.try_get(25)?,
                deleted_at: v.try_get(26)?,
                available_at: v.try_get(27)?,
                year: v.try_get(28)?,
            };

            Ok((prog, book))
        }).collect()
    }
}
