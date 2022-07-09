use common::{PersonId, Either};
use rusqlite::params;

use books_common::MetadataId;
use serde::Serialize;
use crate::{Result, database::Database};

use super::{TableRow, AdvRow};

#[derive(Debug, Serialize)]
pub struct MetadataPersonModel {
	pub metadata_id: MetadataId,
	pub person_id: PersonId,
}


impl TableRow<'_> for MetadataPersonModel {
	fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
		Ok(Self {
			metadata_id: row.next()?,
			person_id: row.next()?,
		})
	}
}

impl MetadataPersonModel {
	pub async fn insert_or_ignore(&self, db: &Database) -> Result<()> {
		db.write().await.execute(
			r#"INSERT OR IGNORE INTO metadata_person (metadata_id, person_id) VALUES (?1, ?2)"#,
			params![
				self.metadata_id,
				self.person_id
			]
		)?;

		Ok(())
	}

	pub async fn delete(&self, db: &Database) -> Result<()> {
		db.write().await.execute(
			r#"DELETE FROM metadata_person WHERE metadata_id = ?1 AND person_id = ?2"#,
			params![
				self.metadata_id,
				self.person_id
			]
		)?;

		Ok(())
	}

	pub async fn delete_by_meta_id(id: MetadataId, db: &Database) -> Result<()> {
		db.write().await.execute(
			r#"DELETE FROM metadata_person WHERE metadata_id = ?1"#,
			[ id ]
		)?;

		Ok(())
	}

	pub async fn delete_by_person_id(id: PersonId, db: &Database) -> Result<()> {
		db.write().await.execute(
			r#"DELETE FROM metadata_person WHERE person_id = ?1"#,
			[ id ]
		)?;

		Ok(())
	}

	pub async fn transfer_person(from_id: PersonId, to_id: PersonId, db: &Database) -> Result<usize> {
		Ok(db.write().await.execute(
			r#"UPDATE metadata_person SET person_id = ?2 WHERE person_id = ?1"#,
			[ from_id, to_id ]
		)?)
	}

	pub async fn find_by(id: Either<MetadataId, PersonId>, db: &Database) -> Result<Vec<Self>> {
		let this = db.read().await;

		match id {
			Either::Left(id) => {
				let mut conn = this.prepare(r#"SELECT * FROM metadata_person WHERE metadata_id = ?1"#)?;

				let map = conn.query_map([ id ], |v| Self::from_row(v))?;

				Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
			}

			Either::Right(id) => {
				let mut conn = this.prepare(r#"SELECT * FROM metadata_person WHERE person_id = ?1"#)?;

				let map = conn.query_map([ id ], |v| Self::from_row(v))?;

				Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
			}
		}
	}
}