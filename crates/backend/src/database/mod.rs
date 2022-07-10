use std::{ops::Deref, sync::Arc};

use crate::Result;
use rusqlite::Connection;
use tokio::sync::{RwLock, Semaphore, SemaphorePermit, RwLockReadGuard, RwLockWriteGuard};
// TODO: use tokio::task::spawn_blocking;

const DATABASE_PATH: &str = "database.db";


pub async fn init() -> Result<Database> {
	let database = Database::open(5, || Ok(Connection::open(DATABASE_PATH)?))?;

	let conn = database.write().await;

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

			"type_of"		INTEGER NOT NULL,

			"permissions"	TEXT NOT NULL,

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


	drop(conn);

	Ok(database)
}


pub struct Database {
	// Using RwLock to engage the r/w locks.
	lock: RwLock<()>,

	// Store all our open connections to the database.
	read_conns: Vec<Arc<Connection>>,

	// Max concurrent read connections
	max_read_aquires: Semaphore,

	// Single-acquire lock to prevent race conditions
	conn_aquire_lock: Semaphore,
}

unsafe impl Send for Database {}
unsafe impl Sync for Database {}

impl Database {
	pub fn open<F: Fn() -> Result<Connection>>(count: usize, open_conn: F) -> Result<Self> {
		let mut read_conns = Vec::new();

		for _ in 0..count {
			read_conns.push(Arc::new(open_conn()?));
		}

		Ok(Self {
			lock: RwLock::new(()),

			read_conns,

			max_read_aquires: Semaphore::new(count),

			conn_aquire_lock: Semaphore::new(1),
		})
	}

	pub async fn read(&self) -> DatabaseReadGuard<'_> {
		// Firstly ensure we can acquire a read lock.
		let _guard = self.lock.read().await;

		// Now we ensure we can acquire another connection
		let _permit = self.max_read_aquires.acquire().await.unwrap();

		let conn = {
			// FIX: A single-acquire quick lock to ensure we don't have race conditions.
			let _temp_lock = self.conn_aquire_lock.acquire().await.unwrap();

			let mut value = None;

			for conn in &self.read_conns {
				let new_conn = conn.clone();
				// Strong count should eq 2 (original + cloned)
				if Arc::strong_count(&new_conn) == 2 {
					value = Some(new_conn);
					break;
				}
			}

			// This should never be reached.
			#[allow(clippy::expect_used)]
			value.expect("Unable to find available Read Connection")
		};

		DatabaseReadGuard {
			_permit,
			_guard,
			conn
		}
	}

	pub async fn write(&self) -> DatabaseWriteGuard<'_> {
		let _guard = self.lock.write().await;

		DatabaseWriteGuard {
			_guard,
			conn: &*self.read_conns[0]
		}
	}
}


pub struct DatabaseReadGuard<'a> {
	_permit: SemaphorePermit<'a>,
	_guard: RwLockReadGuard<'a, ()>,
	conn: Arc<Connection>,
}

impl<'a> Deref for DatabaseReadGuard<'a> {
	type Target = Connection;

	fn deref(&self) -> &Self::Target {
		&*self.conn
	}
}


pub struct DatabaseWriteGuard<'a> {
	_guard: RwLockWriteGuard<'a, ()>,
	conn: &'a Connection,
}

impl<'a> Deref for DatabaseWriteGuard<'a> {
	type Target = Connection;

	fn deref(&self) -> &Self::Target {
		self.conn
	}
}




#[cfg(test)]
mod tests {
	use super::*;

	use std::{thread, time::Duration};
	use tokio::{runtime::Runtime, time::sleep, sync::Mutex};


	fn create_db() -> Result<Arc<Database>> {
		Ok(Arc::new(Database::open(4, || Ok(Connection::open_in_memory()?))?))
	}


	#[test]
	fn write_read() -> Result<()> {
		let database = create_db()?;

		let value = Arc::new(Mutex::new(false));

		let handle_read = {
			let db2 = database.clone();
			let val2 = value.clone();

			thread::spawn(move || {
				Runtime::new().unwrap()
				.block_on(async {
					sleep(Duration::from_millis(100)).await;

					let _read = db2.read().await;

					assert!(*val2.lock().await);
				});
			})
		};



		Runtime::new().unwrap()
		.block_on(async {
			let _write = database.write().await;

			*value.lock().await = true;
		});

		handle_read.join().unwrap();

		Ok(())
	}

	#[test]
	fn multiread_write_read() -> Result<()> {
		let database = create_db()?;

		let value = Arc::new(Mutex::new(false));

		// Create 5 reads
		let handle_reads = (0..5usize)
			.map(|_| {
				let db2 = database.clone();
				let val2 = value.clone();

				thread::spawn(move || {
					Runtime::new().unwrap()
					.block_on(async {
						let _read = db2.read().await;

						sleep(Duration::from_millis(100)).await;

						assert!(!*val2.lock().await);
					});
				})
			})
			.collect::<Vec<_>>();

		// Write
		Runtime::new().unwrap()
		.block_on(async {
			sleep(Duration::from_millis(150)).await;

			let _write = database.write().await;

			*value.lock().await = true;
		});

		for handle_read in handle_reads {
			handle_read.join().unwrap();
		}

		// Read again
		Runtime::new().unwrap()
		.block_on(async {
			let _read = database.read().await;

			assert!(*value.lock().await);
		});

		Ok(())
	}

	// #[test]
	// fn multiple_reads() {}

	// #[test]
	// fn single_writes() {}
}