use std::{ops::Deref, sync::Arc};

use anyhow::Result;
use books_common::StrippedMediaItem;
use rusqlite::{Connection, params, Row, OptionalExtension};
use serde::Serialize;

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
		r#"CREATE TABLE "files" (
			"id" 				INTEGER NOT NULL UNIQUE,

			"path" 				TEXT NOT NULL UNIQUE,
			"file_type" 		TEXT,
			"file_name" 		TEXT NOT NULL,
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

	conn.execute(r#"
		CREATE TABLE "file_notes" (
			"file_id" 		TEXT NOT NULL,
			"user_id" 		TEXT NOT NULL,

			"data" 			TEXT NOT NULL,
			"data_size" 	INTEGER NOT NULL,

			"updated_at" 	DATETIME NOT NULL,
			"created_at" 	DATETIME NOT NULL,

			UNIQUE(file_id, user_id)
		);
	"#, [])?;

	conn.execute(r#"
		CREATE TABLE "file_progression" (
			"file_id" TEXT NOT NULL,
			"user_id" TEXT NOT NULL,

			"chapter" INTEGER NOT NULL,
			"page" INTEGER NOT NULL,

			"updated_at" DATETIME NOT NULL,
			"created_at" DATETIME NOT NULL,

			UNIQUE(file_id, user_id)
		);
	"#, [])?;

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
	pub async fn add_file(&self, file: &NewFile) -> Result<()> {
		self.execute(r#"
			INSERT INTO files (path, file_type, file_name, file_size, modified_at, accessed_at, created_at)
			VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
		"#,
		params![&file.path, &file.file_type, &file.file_name, file.file_size, file.modified_at, file.accessed_at, file.created_at])?;

		Ok(())
	}

	pub async fn list_all_files(&self) -> Result<Vec<File>> {
		let mut conn = self.prepare("SELECT * FROM files")?;

		let map = conn.query_map([], |v| File::try_from(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}

	pub async fn find_file_by_id(&self, id: i64) -> Result<Option<File>> {
		Ok(self.query_row(
			r#"SELECT * FROM files WHERE id=?1 LIMIT 1"#,
			params![id],
			|v| Ok(File::try_from(v))
		).optional()?.transpose()?)
	}

	pub async fn get_file_count(&self) -> Result<i64> {
		Ok(self.query_row(r#"SELECT COUNT(*) FROM files"#, [], |v| v.get(0))?)
	}
}


pub struct NewFile {
	pub path: String,

	pub file_name: String,
	pub file_type: String,
	pub file_size: i64,

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
			file_type: value.get(2)?,
			file_name: value.get(3)?,
			file_size: value.get(4)?,
			modified_at: value.get(5)?,
			accessed_at: value.get(6)?,
			created_at: value.get(7)?,
		})
	}
}


impl From<File> for StrippedMediaItem {
	fn from(val: File) -> Self {
		StrippedMediaItem {
			id: val.id,
			file_name: val.file_name,
			file_type: val.file_type,
			modified_at: val.modified_at,
			created_at: val.created_at,
		}
	}
}