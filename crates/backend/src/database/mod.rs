use std::sync::{Mutex, MutexGuard};

use anyhow::Result;
use books_common::Progression;
use chrono::Utc;
use rusqlite::{Connection, params, OptionalExtension};

pub mod table;
use table::*;


pub async fn init() -> Result<Database> {
	let conn = rusqlite::Connection::open("database.db")?;

	// TODO: Migrations https://github.com/rusqlite/rusqlite/discussions/1117

	// Library
	conn.execute(
		r#"CREATE TABLE IF NOT EXISTS "library" (
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
		r#"CREATE TABLE IF NOT EXISTS "directory" (
			"library_id"	INTEGER NOT NULL,
			"path"			TEXT NOT NULL UNIQUE
		);"#,
		[]
	)?;

	// File
	conn.execute(
		r#"CREATE TABLE IF NOT EXISTS "file" (
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
		r#"CREATE TABLE IF NOT EXISTS "metadata_item" (
			"id"					INTEGER NOT NULL,

			"source"				TEXT,
			"file_item_count"		INTEGER,
			"title"					TEXT,
			"original_title"		TEXT,
			"description"			TEXT,
			"rating"				FLOAT,
			"thumb_url"				TEXT,

			"creator"				TEXT,
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
		r#"CREATE TABLE IF NOT EXISTS "file_note" (
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
		r#"CREATE TABLE IF NOT EXISTS "file_progression" (
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
		r#"CREATE TABLE IF NOT EXISTS "file_notation" (
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

	// Tags People
	conn.execute(
		r#"CREATE TABLE IF NOT EXISTS "tag_person" (
			"id"			INTEGER NOT NULL,

			"source" 		TEXT NOT NULL,
			"type"			INTEGER NOT NULL,

			"name"			TEXT NOT NULL COLLATE NOCASE,
			"description"	TEXT,
			"birth_date"	TEXT,

			"updated_at" 	DATETIME NOT NULL,
			"created_at" 	DATETIME NOT NULL,

			PRIMARY KEY("id" AUTOINCREMENT)
		);"#,
		[]
	)?;

	// People Alt names
	conn.execute(
		r#"CREATE TABLE IF NOT EXISTS "tag_person_alt" (
			"person_id"		INTEGER NOT NULL,

			"name"			TEXT NOT NULL COLLATE NOCASE,

			UNIQUE(person_id, name)
		);"#,
		[]
	)?;

	//

	Ok(Database(Mutex::new(conn)))
}

// TODO: Replace with tokio Mutex?
pub struct Database(Mutex<Connection>);


impl Database {
	fn lock(&self) -> Result<MutexGuard<Connection>> {
		self.0.lock().map_err(|_| anyhow::anyhow!("Database Poisoned"))
	}


	// Libraries

	pub fn add_library(&self, name: String) -> Result<()> {
		// TODO: Create outside of fn.
		let lib = NewLibrary {
			name,
			type_of: String::new(),
			scanned_at: Utc::now(),
			created_at: Utc::now(),
			updated_at: Utc::now(),
		};

		self.lock()?.execute(
			r#"INSERT INTO library (name, type_of, scanned_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4, ?5)"#,
			params![&lib.name, &lib.type_of, lib.scanned_at.timestamp_millis(), lib.created_at.timestamp_millis(), lib.updated_at.timestamp_millis()]
		)?;

		Ok(())
	}

	pub fn remove_library(&self, id: i64) -> Result<usize> {
		self.remove_directories_by_library_id(id)?;

		Ok(self.lock()?.execute(
			r#"DELETE FROM library WHERE id = ?1"#,
			params![id]
		)?)
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

	pub fn remove_directory(&self, path: &str) -> Result<usize> {
		Ok(self.lock()?.execute(
			r#"DELETE FROM directory WHERE path = ?1"#,
			params![path]
		)?)
	}

	pub fn remove_directories_by_library_id(&self, id: i64) -> Result<usize> {
		Ok(self.lock()?.execute(
			r#"DELETE FROM directory WHERE library_id = ?1"#,
			params![id]
		)?)
	}

	pub fn get_directories(&self, library_id: i64) -> Result<Vec<Directory>> {
		let this = self.lock()?;

		let mut conn = this.prepare("SELECT * FROM directory WHERE library_id = ?1")?;

		let map = conn.query_map([library_id], |v| Directory::try_from(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}

	pub fn get_all_directories(&self) -> Result<Vec<Directory>> {
		let this = self.lock()?;

		let mut conn = this.prepare("SELECT * FROM directory")?;

		let map = conn.query_map([], |v| Directory::try_from(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}


	// Files

	pub fn add_file(&self, file: &NewFile) -> Result<()> {
		self.lock()?.execute(r#"
			INSERT INTO file (path, file_type, file_name, file_size, modified_at, accessed_at, created_at, library_id, metadata_id, chapter_count)
			VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)
		"#,
		params![
			&file.path, &file.file_type, &file.file_name, file.file_size,
			file.modified_at.timestamp_millis(), file.accessed_at.timestamp_millis(), file.created_at.timestamp_millis(),
			file.library_id, file.metadata_id, file.chapter_count
		])?;

		Ok(())
	}

	pub fn file_exist(&self, file: &NewFile) -> Result<bool> {
		Ok(self.lock()?.query_row(r#"SELECT id FROM file WHERE path = ?1"#, [&file.path], |_| Ok(1)).optional()?.is_some())
	}

	pub fn get_files_by(&self, library: usize, offset: usize, limit: usize) -> Result<Vec<File>> {
		let this = self.lock()?;

		let mut conn = this.prepare("SELECT * FROM file WHERE library_id = ?1  LIMIT ?2 OFFSET ?3")?;

		let map = conn.query_map([library, limit, offset], |v| File::try_from(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}

	pub fn get_files_with_metadata_by(&self, library: usize, offset: usize, limit: usize) -> Result<Vec<FileWithMetadata>> {
		let this = self.lock()?;

		let mut conn = this.prepare(r#"
			SELECT * FROM file
			LEFT JOIN metadata_item ON metadata_item.id = file.metadata_id
			WHERE library_id = ?1
			LIMIT ?2
			OFFSET ?3
		"#)?;

		let map = conn.query_map([library, limit, offset], |v| FileWithMetadata::try_from(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}

	pub fn get_files_of_no_metadata(&self) -> Result<Vec<File>> {
		let this = self.lock()?;

		let mut conn = this.prepare("SELECT * FROM file WHERE metadata_id = 0 OR metadata_id = NULL")?;

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

	pub fn find_file_by_id_with_metadata(&self, id: i64) -> Result<Option<FileWithMetadata>> {
		Ok(self.lock()?.query_row(
			r#"SELECT * FROM file LEFT JOIN metadata_item ON metadata_item.id = file.metadata_id WHERE file.id = ?1"#,
			[id],
			|v| FileWithMetadata::try_from(v)
		).optional()?)
	}

	pub fn get_file_count(&self) -> Result<i64> {
		Ok(self.lock()?.query_row(r#"SELECT COUNT(*) FROM file"#, [], |v| v.get(0))?)
	}

	pub fn update_file_metadata_id(&self, file_id: i64, metadata_id: i64) -> Result<()> {
		self.lock()?
		.execute(r#"UPDATE file SET metadata_id = ?1 WHERE id = ?2"#,
			params![metadata_id, file_id]
		)?;

		Ok(())
	}

	// Progression

	pub fn add_or_update_progress(&self, user_id: i64, file_id: i64, progress: Progression) -> Result<()> {
		let prog = FileProgression::new(progress, user_id, file_id);

		if self.get_progress(user_id, file_id)?.is_some() {
			self.lock()?.execute(
				r#"UPDATE file_progression SET chapter = ?1, char_pos = ?2, page = ?3, seek_pos = ?4, updated_at = ?5 WHERE file_id = ?6 AND user_id = ?7"#,
				params![prog.chapter, prog.char_pos, prog.page, prog.seek_pos, prog.updated_at.timestamp_millis(), prog.file_id, prog.user_id]
			)?;
		} else {
			self.lock()?.execute(
				r#"INSERT INTO file_progression (file_id, user_id, type_of, chapter, char_pos, page, seek_pos, updated_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)"#,
				params![prog.file_id, prog.user_id, prog.type_of, prog.chapter, prog.char_pos, prog.page, prog.seek_pos, prog.updated_at.timestamp_millis(), prog.created_at.timestamp_millis()]
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
				r#"UPDATE file_note SET data = ?1, data_size = ?2, updated_at = ?3 WHERE file_id = ?4 AND user_id = ?5"#,
				params![prog.data, prog.data_size, prog.updated_at.timestamp_millis(), prog.file_id, prog.user_id]
			)?;
		} else {
			self.lock()?.execute(
				r#"INSERT INTO file_note (file_id, user_id, data, data_size, updated_at, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)"#,
				params![prog.file_id, prog.user_id, prog.data, prog.data_size, prog.updated_at.timestamp_millis(), prog.created_at.timestamp_millis()]
			)?;
		}

		Ok(())
	}

	pub fn get_notes(&self, user_id: i64, file_id: i64) -> Result<Option<FileNote>> {
		Ok(self.lock()?.query_row(
			"SELECT * FROM file_note WHERE user_id = ?1 AND file_id = ?2",
			params![user_id, file_id],
			|v| FileNote::try_from(v)
		).optional()?)
	}

	pub fn delete_notes(&self, user_id: i64, file_id: i64) -> Result<()> {
		self.lock()?.execute(
			"DELETE FROM file_note WHERE user_id = ?1 AND file_id = ?2",
			params![user_id, file_id]
		)?;

		Ok(())
	}


	// Metadata

	pub fn add_or_increment_metadata(&self, meta: &MetadataItem) -> Result<MetadataItem> {
		let table_meta = if meta.id != 0 {
			self.get_metadata_by_id(meta.id)?
		} else {
			self.get_metadata_by_source(&meta.source)?
		};

		if table_meta.is_none() {
			self.lock()?
			.execute(r#"
				INSERT INTO metadata_item (
					source, file_item_count, title, original_title, description, rating, thumb_url,
					creator, publisher,
					tags_genre, tags_collection, tags_author, tags_country,
					available_at, year,
					refreshed_at, created_at, updated_at, deleted_at,
					hash
				)
				VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)"#,
				params![
					&meta.source, &meta.file_item_count, &meta.title, &meta.original_title, &meta.description, &meta.rating, &meta.thumb_url,
					&meta.creator, &meta.publisher,
					&meta.tags_genre, &meta.tags_collection, &meta.tags_author, &meta.tags_country,
					&meta.available_at, &meta.year,
					&meta.refreshed_at.timestamp_millis(), &meta.created_at.timestamp_millis(), &meta.updated_at.timestamp_millis(),
					meta.deleted_at.as_ref().map(|v| v.timestamp_millis()),
					&meta.hash
				]
			)?;

			return Ok(self.get_metadata_by_source(&meta.source)?.unwrap());
		} else if meta.id != 0 {
			self.lock()?
			.execute(r#"UPDATE metadata_item SET file_item_count = file_item_count + 1 WHERE id = ?1"#,
				params![meta.id]
			)?;
		} else {
			self.lock()?
			.execute(r#"UPDATE metadata_item SET file_item_count = file_item_count + 1 WHERE source = ?1"#,
				params![&meta.source]
			)?;
		}

		Ok(table_meta.unwrap())
	}

	pub fn update_metadata(&self, meta: &MetadataItem) -> Result<()> {
		self.lock()?
		.execute(r#"
			UPDATE metadata_item SET
				source = ?2, file_item_count = ?3, title = ?4, original_title = ?5, description = ?6, rating = ?7, thumb_url = ?8,
				creator = ?9, publisher = ?10,
				tags_genre = ?11, tags_collection = ?12, tags_author = ?13, tags_country = ?14,
				available_at = ?15, year = ?16,
				refreshed_at = ?17, created_at = ?18, updated_at = ?19, deleted_at = ?20,
				hash = ?21
			WHERE id = ?1"#,
			params![
				meta.id,
				&meta.source, &meta.file_item_count, &meta.title, &meta.original_title, &meta.description, &meta.rating, &meta.thumb_url,
				&meta.creator, &meta.publisher,
				&meta.tags_genre, &meta.tags_collection, &meta.tags_author, &meta.tags_country,
				&meta.available_at, &meta.year,
				&meta.refreshed_at.timestamp_millis(), &meta.created_at.timestamp_millis(), &meta.updated_at.timestamp_millis(),
				meta.deleted_at.as_ref().map(|v| v.timestamp_millis()),
				&meta.hash
			]
		)?;

		Ok(())
	}

	pub fn decrement_or_remove_metadata(&self, id: i64) -> Result<()> {
		if let Some(meta) = self.get_metadata_by_id(id)? {
			if meta.file_item_count < 1 {
				self.lock()?
				.execute(
					r#"UPDATE metadata_item SET file_item_count = file_item_count - 1 WHERE id = ?1"#,
					params![id]
				)?;
			} else {
				self.lock()?
				.execute(
					r#"DELETE FROM metadata_item WHERE id = ?1"#,
					params![id]
				)?;
			}
		}

		Ok(())
	}

	pub fn decrement_metadata(&self, id: i64) -> Result<()> {
		if let Some(meta) = self.get_metadata_by_id(id)? {
			if meta.file_item_count > 0 {
				self.lock()?
				.execute(
					r#"UPDATE metadata_item SET file_item_count = file_item_count - 1 WHERE id = ?1"#,
					params![id]
				)?;
			}
		}

		Ok(())
	}

	// TODO: Change to get_metadata_by_hash. We shouldn't get metadata by source. Local metadata could be different with the same source id.
	pub fn get_metadata_by_source(&self, source: &str) -> Result<Option<MetadataItem>> {
		Ok(self.lock()?.query_row(
			r#"SELECT * FROM metadata_item WHERE source = ?1 LIMIT 1"#,
			params![source],
			|v| MetadataItem::try_from(v)
		).optional()?)
	}

	pub fn get_metadata_by_id(&self, id: i64) -> Result<Option<MetadataItem>> {
		Ok(self.lock()?.query_row(
			r#"SELECT * FROM metadata_item WHERE id = ?1 LIMIT 1"#,
			params![id],
			|v| MetadataItem::try_from(v)
		).optional()?)
	}


	// Person

	pub fn add_person(&self, person: &NewTagPerson) -> Result<i64> {
		let conn = self.lock()?;

		conn.execute(r#"
			INSERT INTO tag_person (source, type, name, description, birth_date, updated_at, created_at)
			VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
		"#,
		params![
			&person.source, &person.type_of, &person.name, &person.description, &person.birth_date,
			person.updated_at.timestamp_millis(), person.created_at.timestamp_millis()
		])?;

		Ok(conn.last_insert_rowid())
	}

	pub fn get_person_by_name(&self, value: &str) -> Result<Option<TagPerson>> {
		let person = self.lock()?.query_row(
			r#"SELECT * FROM tag_person WHERE name = ?1 LIMIT 1"#,
			params![value],
			|v| TagPerson::try_from(v)
		).optional()?;

		if let Some(person) = person {
			Ok(Some(person))
		} else if let Some(alt) = self.get_person_alt_by_name(value)? {
			self.get_person_by_id(alt.person_id)
		} else {
			Ok(None)
		}
	}

	pub fn get_person_by_id(&self, value: i64) -> Result<Option<TagPerson>> {
		Ok(self.lock()?.query_row(
			r#"SELECT * FROM tag_person WHERE id = ?1 LIMIT 1"#,
			params![value],
			|v| TagPerson::try_from(v)
		).optional()?)
	}

	pub fn get_person_by_source(&self, value: &str) -> Result<Option<TagPerson>> {
		Ok(self.lock()?.query_row(
			r#"SELECT * FROM tag_person WHERE source = ?1 LIMIT 1"#,
			params![value],
			|v| TagPerson::try_from(v)
		).optional()?)
	}


	// Person Alt

	pub fn add_person_alt(&self, person: &TagPersonAlt) -> Result<()> {
		self.lock()?.execute(r#"INSERT INTO tag_person_alt (name, person_id) VALUES (?1, ?2)"#,
		params![
			&person.name, &person.person_id
		])?;

		Ok(())
	}

	pub fn get_person_alt_by_name(&self, value: &str) -> Result<Option<TagPersonAlt>> {
		Ok(self.lock()?.query_row(
			r#"SELECT * FROM tag_person_alt WHERE name = ?1 LIMIT 1"#,
			params![value],
			|v| TagPersonAlt::try_from(v)
		).optional()?)
	}
}