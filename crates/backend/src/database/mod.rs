use std::sync::{Mutex, MutexGuard};

use crate::{Result, model::{metadata::MetadataModel, TableRow}};
use books_common::{api, LibraryId};
use common::{MemberId, PersonId};
use rusqlite::{Connection, params, OptionalExtension};
// TODO: use tokio::task::spawn_blocking;

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

			"identifier" 		TEXT,

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

			"library_id" 			INTEGER,

			"source"				TEXT,
			"file_item_count"		INTEGER,
			"title"					TEXT,
			"original_title"		TEXT,
			"description"			TEXT,
			"rating"				FLOAT,
			"thumb_url"				TEXT,

			"cached"				TEXT,

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

	// Metadata People
	conn.execute(
		r#"CREATE TABLE IF NOT EXISTS "metadata_person" (
			"metadata_id"	INTEGER NOT NULL,
			"person_id"		INTEGER NOT NULL,

			UNIQUE(metadata_id, person_id)
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

			"name"			TEXT NOT NULL COLLATE NOCASE,
			"description"	TEXT,
			"birth_date"	TEXT,

			"thumb_url"		TEXT,

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

	// Members
	conn.execute(
		r#"CREATE TABLE IF NOT EXISTS "members" (
			"id"			INTEGER NOT NULL,

			"name"			TEXT NOT NULL COLLATE NOCASE,
			"email"			TEXT COLLATE NOCASE,
			"password"		TEXT,
			"is_local"		INTEGER NOT NULL,
			"config"		TEXT,

			"created_at" 	DATETIME NOT NULL,
			"updated_at" 	DATETIME NOT NULL,

			UNIQUE(email),
			PRIMARY KEY("id" AUTOINCREMENT)
		);"#,
		[]
	)?;

	// Auths
	conn.execute(
		r#"CREATE TABLE IF NOT EXISTS "auths" (
			"oauth_token"			TEXT NOT NULL,
			"oauth_token_secret"	TEXT NOT NULL,

			"created_at"			DATETIME NOT NULL,

			UNIQUE(oauth_token)
		);"#,
		[]
	)?;


	// Uploaded Images
	conn.execute(
		r#"CREATE TABLE IF NOT EXISTS "uploaded_images" (
			"id"			INTEGER NOT NULL,

			"path"			TEXT NOT NULL,

			"created_at"	DATETIME NOT NULL,

			UNIQUE(path),
			PRIMARY KEY("id" AUTOINCREMENT)
		);"#,
		[]
	)?;

	// Image Link
	conn.execute(
		r#"CREATE TABLE IF NOT EXISTS "image_link" (
			"image_id"		INTEGER NOT NULL,

			"link_id"		INTEGER NOT NULL,
			"type_of"		INTEGER NOT NULL,

			UNIQUE(image_id, link_id, type_of)
		);"#,
		[]
	)?;


	Ok(Database(Mutex::new(conn)))
}

// TODO: Replace with tokio Mutex?
pub struct Database(Mutex<Connection>);


impl Database {
	fn lock(&self) -> Result<MutexGuard<Connection>> {
		Ok(self.0.lock()?)
	}

	// TODO: Preparing for Transfer.
	pub fn read(&self) -> Result<MutexGuard<Connection>> {
		Ok(self.0.lock()?)
	}

	pub fn write(&self) -> Result<MutexGuard<Connection>> {
		Ok(self.0.lock()?)
	}


	// Search
	fn gen_search_query(search: &api::SearchQuery, library: Option<LibraryId>) -> Option<String> {
		let mut sql = String::from("SELECT * FROM metadata_item WHERE ");
		let orig_len = sql.len();

		// Library ID

		if let Some(library) = library {
			sql += &format!("library_id={} ", library);
		}


		// Query

		if let Some(query) = search.query.as_deref() {
			if library.is_some() {
				sql += "AND ";
			}

			let mut escape_char = '\\';
			// Change our escape character if it's in the query.
			if query.contains(escape_char) {
				for car in [ '!', '@', '#', '$', '^', '&', '*', '-', '=', '+', '|', '~', '`', '/', '?', '>', '<', ',' ] {
					if !query.contains(car) {
						escape_char = car;
						break;
					}
				}
			}

			// TODO: Utilize title > original_title > description, and sort
			sql += &format!(
				"title LIKE '%{}%' ESCAPE '{}' ",
				query.replace('%', &format!("{}%", escape_char)).replace('_', &format!("{}_", escape_char)),
				escape_char
			);
		}


		// Source

		if let Some(source) = search.source.as_deref() {
			if search.query.is_some() || library.is_some() {
				sql += "AND ";
			}

			sql += &format!("source LIKE '{}%' ", source);
		}

		if sql.len() == orig_len {
			// If sql is still unmodified
			None
		} else {
			Some(sql)
		}
	}

	pub fn search_metadata_list(&self, search: &api::SearchQuery, library: Option<LibraryId>, offset: usize, limit: usize) -> Result<Vec<MetadataModel>> {
		let mut sql = match Self::gen_search_query(search, library) {
			Some(v) => v,
			None => return Ok(Vec::new())
		};

		sql += "LIMIT ?1 OFFSET ?2";

		let this = self.lock()?;

		let mut conn = this.prepare(&sql)?;

		let map = conn.query_map(params![limit, offset], |v| MetadataModel::from_row(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}

	pub fn count_search_metadata(&self, search: &api::SearchQuery, library: Option<LibraryId>) -> Result<usize> {
		let sql = match Self::gen_search_query(search, library) {
			Some(v) => v.replace("SELECT *", "SELECT COUNT(*)"),
			None => return Ok(0)
		};

		Ok(self.lock()?.query_row(&sql, [], |v| v.get(0))?)
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

	pub fn remove_person_alt(&self, tag_person: &TagPersonAlt) -> Result<usize> {
		Ok(self.lock()?.execute(
			r#"DELETE FROM tag_person_alt WHERE name = ?1 AND person_id = ?2"#,
			params![
				&tag_person.name,
				&tag_person.person_id
			]
		)?)
	}

	pub fn remove_person_alt_by_person_id(&self, id: PersonId) -> Result<usize> {
		Ok(self.lock()?.execute(
			r#"DELETE FROM tag_person_alt WHERE person_id = ?1"#,
			params![id]
		)?)
	}

	pub fn transfer_person_alt(&self, from_id: PersonId, to_id: PersonId) -> Result<usize> {
		Ok(self.lock()?.execute(r#"UPDATE OR IGNORE tag_person_alt SET person_id = ?2 WHERE person_id = ?1"#,
		params![
			from_id,
			to_id
		])?)
	}


	// Members

	pub fn add_member(&self, member: &NewMember) -> Result<MemberId> {
		let conn = self.lock()?;

		conn.execute(r#"
			INSERT INTO members (name, email, password, is_local, config, created_at, updated_at)
			VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
		"#,
		params![
			&member.name, member.email.as_ref(), member.password.as_ref(), member.type_of, member.config.as_ref(),
			member.created_at.timestamp_millis(), member.updated_at.timestamp_millis()
		])?;

		Ok(MemberId::from(conn.last_insert_rowid() as usize))
	}

	pub fn get_member_by_email(&self, value: &str) -> Result<Option<Member>> {
		Ok(self.lock()?.query_row(
			r#"SELECT * FROM members WHERE email = ?1 LIMIT 1"#,
			params![value],
			|v| Member::try_from(v)
		).optional()?)
	}

	pub fn get_member_by_id(&self, id: MemberId) -> Result<Option<Member>> {
		Ok(self.lock()?.query_row(
			r#"SELECT * FROM members WHERE id = ?1 LIMIT 1"#,
			params![id],
			|v| Member::try_from(v)
		).optional()?)
	}


	// Verify

	pub fn add_verify(&self, auth: &NewAuth) -> Result<usize> {
		let conn = self.lock()?;

		conn.execute(r#"
			INSERT INTO auths (oauth_token, oauth_token_secret, created_at)
			VALUES (?1, ?2, ?3)
		"#,
		params![
			&auth.oauth_token,
			&auth.oauth_token_secret,
			auth.created_at.timestamp_millis()
		])?;

		Ok(conn.last_insert_rowid() as usize)
	}

	pub fn remove_verify_if_found_by_oauth_token(&self, value: &str) -> Result<bool> {
		Ok(self.lock()?.execute(
			r#"DELETE FROM auths WHERE oauth_token = ?1 LIMIT 1"#,
			params![value],
		)? != 0)
	}
}