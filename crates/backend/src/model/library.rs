use chrono::{DateTime, TimeZone, Utc};
use rusqlite::{params, OptionalExtension};
use serde::Serialize;

use common_local::{LibraryId, util::serialize_datetime};
use crate::{Result, database::Database};

use super::{TableRow, AdvRow, directory::DirectoryModel};



pub struct NewLibraryModel {
	pub name: String,

	pub scanned_at: DateTime<Utc>,
	pub created_at: DateTime<Utc>,
	pub updated_at: DateTime<Utc>,
}


#[derive(Debug, Serialize)]
pub struct LibraryModel {
	pub id: LibraryId,

	pub name: String,

	#[serde(serialize_with = "serialize_datetime")]
	pub scanned_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub created_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime")]
	pub updated_at: DateTime<Utc>,
}



impl TableRow<'_> for LibraryModel {
	fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
		Ok(Self {
			id: row.next()?,
			name: row.next()?,
			scanned_at: Utc.timestamp_millis(row.next()?),
			created_at: Utc.timestamp_millis(row.next()?),
			updated_at: Utc.timestamp_millis(row.next()?),
		})
	}
}



impl NewLibraryModel {
	pub async fn insert(self, db: &Database) -> Result<LibraryModel> {
		let lock = db.write().await;

		lock.execute(
			r#"INSERT INTO library (name, scanned_at, created_at, updated_at) VALUES (?1, ?2, ?3, ?4)"#,
			params![
				&self.name,
				self.scanned_at.timestamp_millis(),
				self.created_at.timestamp_millis(),
				self.updated_at.timestamp_millis()
			]
		)?;

		Ok(LibraryModel {
			id: LibraryId::from(lock.last_insert_rowid() as usize),
			name: self.name,
			scanned_at: self.scanned_at,
			created_at: self.created_at,
			updated_at: self.updated_at,
		})
	}
}


impl LibraryModel {
	pub async fn delete_by_id(id: LibraryId, db: &Database) -> Result<usize> {
		DirectoryModel::delete_by_library_id(id, db).await?;

		Ok(db.write().await.execute(r#"DELETE FROM library WHERE id = ?1"#, [id])?)
	}

	pub async fn get_all(db: &Database) -> Result<Vec<LibraryModel>> {
		let this = db.read().await;

		let mut conn = this.prepare("SELECT * FROM library")?;

		let map = conn.query_map([], |v| LibraryModel::from_row(v))?;

		Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
	}

	pub async fn find_one_by_name(value: &str, db: &Database) -> Result<Option<LibraryModel>> {
		Ok(db.read().await.query_row(
			r#"SELECT * FROM library WHERE name = ?1 LIMIT 1"#,
			params![value],
			|v| LibraryModel::from_row(v)
		).optional()?)
	}
}