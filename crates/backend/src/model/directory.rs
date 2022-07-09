use rusqlite::params;

use books_common::LibraryId;
use crate::{Result, database::Database};

use super::{TableRow, AdvRow};





pub struct DirectoryModel {
	pub library_id: LibraryId,
	pub path: String,
}


impl TableRow<'_> for DirectoryModel {
	fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
		Ok(Self {
			library_id: row.next()?,
			path: row.next()?,
		})
	}
}


impl DirectoryModel {
	pub fn insert(&self, db: &Database) -> Result<()> {
		db.write()?.execute(
			r#"INSERT INTO directory (library_id, path) VALUES (?1, ?2)"#,
			params![&self.library_id, &self.path]
		)?;

		Ok(())
	}

	pub fn remove_by_path(path: &str, db: &Database) -> Result<usize> {
		Ok(db.write()?.execute(
			r#"DELETE FROM directory WHERE path = ?1"#,
			[path]
		)?)
	}

	pub fn remove_by_library_id(id: LibraryId, db: &Database) -> Result<usize> {
		Ok(db.write()?.execute(
			r#"DELETE FROM directory WHERE library_id = ?1"#,
			[id]
		)?)
	}

	pub fn get_directories(library_id: LibraryId, db: &Database) -> Result<Vec<DirectoryModel>> {
		let this = db.read()?;

		let mut conn = this.prepare("SELECT * FROM directory WHERE library_id = ?1")?;

		let map = conn.query_map([library_id], |v| DirectoryModel::from_row(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}

	pub fn get_all(db: &Database) -> Result<Vec<DirectoryModel>> {
		let this = db.read()?;

		let mut conn = this.prepare("SELECT * FROM directory")?;

		let map = conn.query_map([], |v| DirectoryModel::from_row(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}
}