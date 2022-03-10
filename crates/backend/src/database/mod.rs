use std::{ops::Deref, sync::Arc};

use anyhow::Result;
use books_common::Progression;
use chrono::{DateTime, Utc};
use rusqlite::{Connection, params, Row, OptionalExtension};
use serde::{Serialize, Serializer};

pub async fn init() -> Result<Database> {
	let _ = tokio::fs::remove_file("database.db").await;
	let conn = rusqlite::Connection::open("database.db")?;

	// TODO: Multiple library paths.
	conn.execute(
		r#"CREATE TABLE "library" (
			"id" 				INTEGER NOT NULL UNIQUE,

			"name" 				TEXT,
			"type_of" 			TEXT,

			"path" 				TEXT NOT NULL UNIQUE,

			"scanned_at" 		DATETIME NOT NULL,
			"created_at" 		DATETIME NOT NULL,
			"updated_at" 		DATETIME NOT NULL,

			PRIMARY KEY("id" AUTOINCREMENT)
		);"#,
		[]
	)?;

	conn.execute(
		r#"CREATE TABLE "file" (
			"id" 				INTEGER NOT NULL UNIQUE,

			"path" 				TEXT NOT NULL UNIQUE,
			"file_name" 		TEXT NOT NULL,
			"file_type" 		TEXT,
			"file_size" 		INTEGER NOT NULL,

			"library_id" 		INTEGER,
			"metadata_id" 		INTEGER,
			"chapter_count" 	INTEGER,

			"modified_at" 		DATETIME NOT NULL,
			"accessed_at" 		DATETIME NOT NULL,
			"created_at" 		DATETIME NOT NULL,

			PRIMARY KEY("id" AUTOINCREMENT)
		);"#,
		[]
	)?;

	conn.execute(
		r#"CREATE TABLE "metadata_items" (
			"id"					INTEGER NOT NULL,

			"guid"					TEXT,
			"file_item_count"		INTEGER,
			"title"					TEXT,
			"original_title"		TEXT,
			"description"			TEXT,
			"rating"				FLOAT,
			"thumb_url"				TEXT,

			"publisher"				TEXT,
			"tags_genre"			TEXT,
			"tags_collection"		TEXT,
			"tags_author"			TEXT,
			"tags_country"			TEXT,

			"available_at"			DATETIME,
			"year"					INTEGER,

			"refreshed_at"			DATETIME,
			"created_at"			DATETIME,
			"updated_at"			DATETIME,
			"deleted_at"			DATETIME,

			"hash"					TEXT,

			PRIMARY KEY("id" AUTOINCREMENT)
		);"#,
		[]
	)?;

	conn.execute(
		r#"CREATE TABLE "file_notes" (
			"file_id" 		TEXT NOT NULL,
			"user_id" 		TEXT NOT NULL,

			"data" 			TEXT NOT NULL,
			"data_size" 	INTEGER NOT NULL,

			"updated_at" 	DATETIME NOT NULL,
			"created_at" 	DATETIME NOT NULL,

			UNIQUE(file_id, user_id)
		);"#,
		[]
	)?;

	conn.execute(
		r#"CREATE TABLE "file_progression" (
			"file_id" TEXT NOT NULL,
			"user_id" TEXT NOT NULL,

			"chapter" INTEGER NOT NULL,
			"page" INTEGER NOT NULL,

			"updated_at" DATETIME NOT NULL,
			"created_at" DATETIME NOT NULL,

			UNIQUE(file_id, user_id)
		);"#,
		[]
	)?;

	Ok(Database(Arc::new(conn)))
}


#[derive(Clone)]
pub struct Database(Arc<Connection>);

unsafe impl Sync for Database {}
#[allow(clippy::non_send_fields_in_send_ty)]
unsafe impl Send for Database {}

impl Deref for Database {
    type Target = Connection;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Database {
	// Libraries
	pub fn add_library(&self, path: &str) -> Result<()> {
		// TODO: Create outside of fn.
		let lib = NewLibrary {
			name: String::from("Books"),
			type_of: String::new(),
			path: path.to_string(),
			scanned_at: Utc::now(),
			created_at: Utc::now(),
			updated_at: Utc::now(),
		};

		self.execute(
			r#"INSERT INTO library (name, type_of, path, scanned_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
			params![&lib.name, &lib.type_of, &lib.path, lib.scanned_at, lib.created_at, lib.updated_at]
		)?;

		Ok(())
	}

	pub fn list_all_libraries(&self) -> Result<Vec<Library>> {
		let mut conn = self.prepare("SELECT * FROM library")?;

		let map = conn.query_map([], |v| Library::try_from(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}

	// Files
	pub fn add_file(&self, file: &NewFile) -> Result<()> {
		self.execute(r#"
			INSERT INTO file (path, file_type, file_name, file_size, modified_at, accessed_at, created_at, library_id, metadata_id, chapter_count)
			VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
		"#,
		params![&file.path, &file.file_type, &file.file_name, file.file_size, file.modified_at, file.accessed_at, file.created_at, file.library_id, file.metadata_id, file.chapter_count])?;

		Ok(())
	}

	pub fn list_all_files(&self) -> Result<Vec<File>> {
		let mut conn = self.prepare("SELECT * FROM file")?;

		let map = conn.query_map([], |v| File::try_from(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}

	pub fn find_file_by_id(&self, id: i64) -> Result<Option<File>> {
		Ok(self.query_row(
			r#"SELECT * FROM file WHERE id=?1 LIMIT 1"#,
			params![id],
			|v| Ok(File::try_from(v))
		).optional()?.transpose()?)
	}

	pub fn get_file_count(&self) -> Result<i64> {
		Ok(self.query_row(r#"SELECT COUNT(*) FROM file"#, [], |v| v.get(0))?)
	}

	// Progression
	pub fn add_or_update_progress(&self, user_id: i64, file_id: i64, progress: Progression) -> Result<()> {
		let prog = FileProgression::new(progress, user_id, file_id);

		if self.get_progress(user_id, file_id)?.is_some() {
			self.execute(
				r#"UPDATE file_progression SET chapter = ?1, page = ?2, seek_pos = ?3, updated_at = ?4 WHERE file_id = ?5 AND user_id = ?6 LIMIT 1"#,
				params![prog.chapter, prog.page, prog.seek_pos, prog.updated_at, prog.file_id, prog.user_id]
			)?;
		} else {
			self.execute(
				r#"INSERT INTO file_progression (file_id, user_id, type_of, chapter, page, seek_pos, updated_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)"#,
				params![prog.file_id, prog.user_id, prog.type_of, prog.chapter, prog.page, prog.seek_pos, prog.updated_at, prog.created_at]
			)?;
		}

		Ok(())
	}

	pub fn get_progress(&self, user_id: i64, file_id: i64) -> Result<Option<FileProgression>> {
		Ok(self.query_row(
			"SELECT * FROM file_progression WHERE user_id = ?1 AND file_id = ?2",
			params![user_id, file_id],
			|v| FileProgression::try_from(v)
		).optional()?)
	}
}


// TODO: Move to another file.

#[derive(Debug, Serialize)]
pub struct MetadataItem {
	pub id: i64,

	pub guid: String,
	pub file_item_count: i64,
	pub title: String,
	pub original_title: String,
	pub description: String,
	pub rating: f64,
	pub thumb_url: String,

	pub publisher: String,
	pub tags_genre: String,
	pub tags_collection: String,
	pub tags_author: String,
	pub tags_country: String,

	pub refreshed_at: i64,
	pub created_at: i64,
	pub updated_at: i64,
	pub deleted_at: i64,

	pub available_at: i64,
	pub year: i64,

	pub hash: String
}


#[derive(Debug, Serialize)]
pub struct FileNote {
	pub file_id: i64,
	pub user_id: i64,

	pub data: String,
	pub data_size: i64,

	pub updated_at: i64,
	pub created_at: i64,
}


#[derive(Debug, Serialize)]
pub struct FileProgression {
	pub file_id: i64,
	pub user_id: i64,

	pub type_of: u8,

	// Ebook/Audiobook
	pub chapter: i64,

	// Ebook
	pub page: Option<i64>,

	// Audiobook
	pub seek_pos: Option<i64>,

	#[serde(serialize_with = "serialize_datetime")]
	pub updated_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub created_at: DateTime<Utc>,
}

impl FileProgression {
	pub fn new(progress: Progression, user_id: i64, file_id: i64) -> Self {
		match progress {
			Progression::Ebook { chapter, page } => Self {
				file_id,
				user_id,
				type_of: 0,
				chapter,
				page: Some(page),
				seek_pos: None,
				updated_at: Utc::now(),
				created_at: Utc::now(),
			},

			Progression::AudioBook { chapter, seek_pos } => Self {
				file_id,
				user_id,
				type_of: 1,
				chapter,
				page: None,
				seek_pos: Some(seek_pos),
				updated_at: Utc::now(),
				created_at: Utc::now(),
			}
		}
	}
}

impl<'a> TryFrom<&Row<'a>> for FileProgression {
	type Error = rusqlite::Error;

	fn try_from(value: &Row<'a>) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			file_id: value.get(0)?,
			user_id: value.get(1)?,

			type_of: value.get(2)?,

			chapter: value.get(3)?,

			page: value.get(4)?,

			seek_pos: value.get(5)?,

			updated_at: value.get(6)?,
			created_at: value.get(7)?,
		})
	}
}

impl From<FileProgression> for Progression {
    fn from(val: FileProgression) -> Self {
        match val.type_of {
			0 => Progression::Ebook {
				chapter: val.chapter,
				page: val.page.unwrap(),
			},

			1 => Progression::AudioBook {
				chapter: val.chapter,
				seek_pos: val.seek_pos.unwrap(),
			},

			_ => unreachable!()
		}
    }
}


pub struct NewLibrary {
	pub name: String,
	pub type_of: String,

	pub path: String,

	pub scanned_at: DateTime<Utc>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct Library {
	pub id: i64,

	pub name: String,
	pub type_of: String,

	pub path: String,

	#[serde(serialize_with = "serialize_datetime")]
	pub scanned_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub created_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub updated_at: DateTime<Utc>,
}

impl<'a> TryFrom<&Row<'a>> for Library {
	type Error = rusqlite::Error;

	fn try_from(value: &Row<'a>) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			id: value.get(0)?,
			name: value.get(1)?,
			type_of: value.get(2)?,
			path: value.get(3)?,
			scanned_at: value.get(4)?,
			created_at: value.get(5)?,
			updated_at: value.get(6)?,
		})
	}
}


pub struct NewFile {
	pub path: String,

	pub file_name: String,
	pub file_type: String,
	pub file_size: i64,

	pub library_id: i64,
	pub metadata_id: i64,
	pub chapter_count: i64,

	pub modified_at: i64,
	pub accessed_at: i64,
	pub created_at: i64,
}

#[derive(Debug, Serialize)]
pub struct File {
	pub id: i64,

	pub path: String,

	pub file_name: String,
	pub file_type: String,
	pub file_size: i64,

	pub library_id: i64,
	pub metadata_id: i64,
	pub chapter_count: i64,

	pub modified_at: i64,
	pub accessed_at: i64,
	pub created_at: i64,
}

impl<'a> TryFrom<&Row<'a>> for File {
	type Error = rusqlite::Error;

	fn try_from(value: &Row<'a>) -> std::result::Result<Self, Self::Error> {
		Ok(Self {
			id: value.get(0)?,

			path: value.get(1)?,

			file_name: value.get(2)?,
			file_type: value.get(3)?,
			file_size: value.get(4)?,

			library_id: value.get(5)?,
			metadata_id: value.get(6)?,
			chapter_count: value.get(7)?,

			modified_at: value.get(8)?,
			accessed_at: value.get(9)?,
			created_at: value.get(10)?,
		})
	}
}


fn serialize_datetime<S>(value: &DateTime<Utc>, s: S) -> std::result::Result<S::Ok, S::Error> where S: Serializer {
	s.serialize_i64(value.timestamp_millis())
}