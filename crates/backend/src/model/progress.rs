use chrono::{DateTime, Utc, TimeZone};
use common::MemberId;
use rusqlite::{params, OptionalExtension};

use books_common::{Progression, util::serialize_datetime, FileId};
use serde::Serialize;
use crate::{Result, database::Database};

use super::{TableRow, AdvRow};




#[derive(Debug, Serialize)]
pub struct FileProgressionModel {
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

	#[serde(serialize_with = "serialize_datetime")]
	pub updated_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub created_at: DateTime<Utc>,
}

impl FileProgressionModel {
	pub fn new(progress: Progression, user_id: MemberId, file_id: FileId) -> Self {
		match progress {
			Progression::Complete => Self {
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

			Progression::Ebook { chapter, page, char_pos } => Self {
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
				file_id,
				user_id,
				type_of: 2,
				chapter: Some(chapter),
				page: None,
				char_pos: None,
				seek_pos: Some(seek_pos),
				updated_at: Utc::now(),
				created_at: Utc::now(),
			}
		}
	}
}


impl TableRow<'_> for FileProgressionModel {
	fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
		Ok(Self {
			file_id: row.next()?,
			user_id: row.next()?,

			type_of: row.next()?,

			chapter: row.next()?,

			page: row.next()?,
			char_pos: row.next()?,

			seek_pos: row.next()?,

			updated_at: Utc.timestamp_millis(row.next()?),
			created_at: Utc.timestamp_millis(row.next()?),
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

			_ => unreachable!()
		}
    }
}



impl FileProgressionModel {
	pub async fn insert_or_update(member_id: MemberId, file_id: FileId, progress: Progression, db: &Database) -> Result<()> {
		let prog = Self::new(progress, member_id, file_id);

		if Self::find_one(member_id, file_id, db).await?.is_some() {
			db.write().await.execute(
				r#"UPDATE file_progression SET chapter = ?1, char_pos = ?2, page = ?3, seek_pos = ?4, updated_at = ?5 WHERE file_id = ?6 AND user_id = ?7"#,
				params![prog.chapter, prog.char_pos, prog.page, prog.seek_pos, prog.updated_at.timestamp_millis(), prog.file_id, prog.user_id]
			)?;
		} else {
			db.write().await.execute(
				r#"INSERT INTO file_progression (file_id, user_id, type_of, chapter, char_pos, page, seek_pos, updated_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
				params![prog.file_id, prog.user_id, prog.type_of, prog.chapter, prog.char_pos, prog.page, prog.seek_pos, prog.updated_at.timestamp_millis(), prog.created_at.timestamp_millis()]
			)?;
		}

		Ok(())
	}

	pub async fn find_one(member_id: MemberId, file_id: FileId, db: &Database) -> Result<Option<Self>> {
		Ok(db.read().await.query_row(
			"SELECT * FROM file_progression WHERE user_id = ?1 AND file_id = ?2",
			params![member_id, file_id],
			|v| Self::from_row(v)
		).optional()?)
	}

	pub async fn delete_one(member_id: MemberId, file_id: FileId, db: &Database) -> Result<()> {
		db.write().await.execute(
			"DELETE FROM file_progression WHERE user_id = ?1 AND file_id = ?2",
			params![member_id, file_id]
		)?;

		Ok(())
	}
}