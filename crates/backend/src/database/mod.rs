use std::sync::{Mutex, MutexGuard};

use anyhow::Result;
use books_common::Progression;
use chrono::Utc;
use rusqlite::{Connection, params, OptionalExtension};

pub mod table;
use table::*;


pub async fn init() -> Result<Database> {
	let _ = tokio::fs::remove_file("database.db").await;
	let conn = rusqlite::Connection::open("database.db")?;

	// TODO: Migrations https://github.com/rusqlite/rusqlite/discussions/1117

	// Library
	conn.execute(
		r#"CREATE TABLE "library" (
			"id" 				INTEGER NOT NULL UNIQUE,

			"name" 				TEXT UNIQUE,
			"type_of" 			TEXT,

			"scanned_at" 		DATETIME NOT NULL,
			"created_at" 		DATETIME NOT NULL,
			"updated_at" 		DATETIME NOT NULL,

			PRIMARY KEY("id" AUTOINCREMENT)
		);"#,
		[]
	)?;

	// Directory
	conn.execute(
		r#"CREATE TABLE "directory" (
			"library_id"	INTEGER NOT NULL,
			"path"			TEXT NOT NULL UNIQUE
		);"#,
		[]
	)?;

	// File
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

	// Metadata Item
	conn.execute(
		r#"CREATE TABLE "metadata_item" (
			"id"					INTEGER NOT NULL,

			"source"				TEXT,
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

	// TODO: Versionize Notes. Keep last 20 versions for X one month. Auto delete old versions.
	// File Note
	conn.execute(
		r#"CREATE TABLE "file_note" (
			"file_id" 		INTEGER NOT NULL,
			"user_id" 		INTEGER NOT NULL,

			"data" 			TEXT NOT NULL,
			"data_size" 	INTEGER NOT NULL,

			"updated_at" 	DATETIME NOT NULL,
			"created_at" 	DATETIME NOT NULL,

			UNIQUE(file_id, user_id)
		);"#,
		[]
	)?;

	// File Progression
	conn.execute(
		r#"CREATE TABLE "file_progression" (
			"file_id" INTEGER NOT NULL,
			"user_id" INTEGER NOT NULL,

			"type_of" INTEGER NOT NULL,

			"chapter" INTEGER,
			"page" INTEGER,
			"char_pos" INTEGER,
			"seek_pos" INTEGER,

			"updated_at" DATETIME NOT NULL,
			"created_at" DATETIME NOT NULL,

			UNIQUE(file_id, user_id)
		);"#,
		[]
	)?;

	// File Notation
	conn.execute(
		r#"CREATE TABLE "file_notation" (
			"file_id" 		INTEGER NOT NULL,
			"user_id" 		INTEGER NOT NULL,

			"data" 			TEXT NOT NULL,
			"data_size" 	INTEGER NOT NULL,

			"updated_at" 	DATETIME NOT NULL,
			"created_at" 	DATETIME NOT NULL,

			UNIQUE(file_id, user_id)
		);"#,
		[]
	)?;

	Ok(Database(Mutex::new(conn)))
}

// TODO: Replace with tokio Mutex?
pub struct Database(Mutex<Connection>);


impl Database {
	fn lock(&self) -> Result<MutexGuard<Connection>> {
		self.0.lock().map_err(|_| anyhow::anyhow!("Database Poisoned"))
	}


	// Libraries
	pub fn add_library(&self, path: &str) -> Result<()> {
		// TODO: Create outside of fn.
		let lib = NewLibrary {
			name: String::from("Books"),
			type_of: String::new(),
			scanned_at: Utc::now(),
			created_at: Utc::now(),
			updated_at: Utc::now(),
		};

		self.lock()?.execute(
			r#"INSERT INTO library (name, type_of, scanned_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)"#,
			params![&lib.name, &lib.type_of, lib.scanned_at, lib.created_at, lib.updated_at]
		)?;

		let lib = self.get_library_by_name("Books")?.unwrap();
		// TODO: Correct.
		self.add_directory(lib.id, path.to_string())?;

		Ok(())
	}

	pub fn list_all_libraries(&self) -> Result<Vec<Library>> {
		let this = self.lock()?;

		let mut conn = this.prepare("SELECT * FROM library")?;

		let map = conn.query_map([], |v| Library::try_from(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}

	pub fn get_library_by_name(&self, value: &str) -> Result<Option<Library>> {
		Ok(self.lock()?.query_row(
			r#"SELECT * FROM library WHERE name = ?1 LIMIT 1"#,
			params![value],
			|v| Library::try_from(v)
		).optional()?)
	}


	// Directories
	pub fn add_directory(&self, library_id: i64, path: String) -> Result<()> {
		self.lock()?.execute(
			r#"INSERT INTO directory (library_id, path) VALUES (?1, ?2)"#,
			params![&library_id, &path]
		)?;

		Ok(())
	}

	pub fn get_directories(&self, library_id: i64) -> Result<Vec<Directory>> {
		let this = self.lock()?;

		let mut conn = this.prepare("SELECT * FROM directory WHERE library_id = ?1")?;

		let map = conn.query_map([library_id], |v| Directory::try_from(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}


	// Files
	pub fn add_file(&self, file: &NewFile) -> Result<()> {
		self.lock()?.execute(r#"
			INSERT INTO file (path, file_type, file_name, file_size, modified_at, accessed_at, created_at, library_id, metadata_id, chapter_count)
			VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
		"#,
		params![&file.path, &file.file_type, &file.file_name, file.file_size, file.modified_at, file.accessed_at, file.created_at, file.library_id, file.metadata_id, file.chapter_count])?;

		Ok(())
	}

	pub fn list_all_files(&self) -> Result<Vec<File>> {
		let this = self.lock()?;

		let mut conn = this.prepare("SELECT * FROM file")?;

		let map = conn.query_map([], |v| File::try_from(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}

	pub fn find_file_by_id(&self, id: i64) -> Result<Option<File>> {
		Ok(self.lock()?.query_row(
			r#"SELECT * FROM file WHERE id=?1 LIMIT 1"#,
			params![id],
			|v| File::try_from(v)
		).optional()?)
	}

	pub fn get_file_count(&self) -> Result<i64> {
		Ok(self.lock()?.query_row(r#"SELECT COUNT(*) FROM file"#, [], |v| v.get(0))?)
	}


	// Progression
	pub fn add_or_update_progress(&self, user_id: i64, file_id: i64, progress: Progression) -> Result<()> {
		let prog = FileProgression::new(progress, user_id, file_id);

		if self.get_progress(user_id, file_id)?.is_some() {
			self.lock()?.execute(
				r#"UPDATE file_progression SET chapter = ?1, char_pos = ?2, page = ?3, seek_pos = ?4, updated_at = ?5 WHERE file_id = ?6 AND user_id = ?7"#,
				params![prog.chapter, prog.char_pos, prog.page, prog.seek_pos, prog.updated_at, prog.file_id, prog.user_id]
			)?;
		} else {
			self.lock()?.execute(
				r#"INSERT INTO file_progression (file_id, user_id, type_of, chapter, char_pos, page, seek_pos, updated_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
				params![prog.file_id, prog.user_id, prog.type_of, prog.chapter, prog.char_pos, prog.page, prog.seek_pos, prog.updated_at, prog.created_at]
			)?;
		}

		Ok(())
	}

	pub fn get_progress(&self, user_id: i64, file_id: i64) -> Result<Option<FileProgression>> {
		Ok(self.lock()?.query_row(
			"SELECT * FROM file_progression WHERE user_id = ?1 AND file_id = ?2",
			params![user_id, file_id],
			|v| FileProgression::try_from(v)
		).optional()?)
	}

	pub fn delete_progress(&self, user_id: i64, file_id: i64) -> Result<()> {
		self.lock()?.execute(
			"DELETE FROM file_progression WHERE user_id = ?1 AND file_id = ?2",
			params![user_id, file_id]
		)?;

		Ok(())
	}


	// Notes
	pub fn add_or_update_notes(&self, user_id: i64, file_id: i64, data: String) -> Result<()> {
		let prog = FileNote::new(file_id, user_id, data);

		if self.get_notes(user_id, file_id)?.is_some() {
			self.lock()?.execute(
				r#"UPDATE file_notes SET data = ?1, data_size = ?2, updated_at = ?3 WHERE file_id = ?4 AND user_id = ?5"#,
				params![prog.data, prog.data_size, prog.updated_at, prog.file_id, prog.user_id]
			)?;
		} else {
			self.lock()?.execute(
				r#"INSERT INTO file_notes (file_id, user_id, data, data_size, updated_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
				params![prog.file_id, prog.user_id, prog.data, prog.data_size, prog.updated_at, prog.created_at]
			)?;
		}

		Ok(())
	}

	pub fn get_notes(&self, user_id: i64, file_id: i64) -> Result<Option<FileNote>> {
		Ok(self.lock()?.query_row(
			"SELECT * FROM file_notes WHERE user_id = ?1 AND file_id = ?2",
			params![user_id, file_id],
			|v| FileNote::try_from(v)
		).optional()?)
	}

	pub fn delete_notes(&self, user_id: i64, file_id: i64) -> Result<()> {
		self.lock()?.execute(
			"DELETE FROM file_notes WHERE user_id = ?1 AND file_id = ?2",
			params![user_id, file_id]
		)?;

		Ok(())
	}
}