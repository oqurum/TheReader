use common::PersonId;
use rusqlite::{params, OptionalExtension};

use serde::Serialize;
use crate::{Result, database::Database};

use super::{TableRow, AdvRow};


#[derive(Debug, Serialize)]
pub struct PersonAltModel {
	pub person_id: PersonId,
	pub name: String,
}


impl TableRow<'_> for PersonAltModel {
	fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
		Ok(Self {
			person_id: row.next()?,
			name: row.next()?,
		})
	}
}


impl PersonAltModel {
	pub fn insert(&self, db: &Database) -> Result<()> {
		db.write()?.execute(
            r#"INSERT INTO tag_person_alt (name, person_id) VALUES (?1, ?2)"#,
            params![
                &self.name, &self.person_id
            ]
        )?;

		Ok(())
	}

	pub fn get_by_name(value: &str, db: &Database) -> Result<Option<Self>> {
		Ok(db.read()?.query_row(
			r#"SELECT * FROM tag_person_alt WHERE name = ?1 LIMIT 1"#,
			params![value],
			|v| Self::from_row(v)
		).optional()?)
	}

	pub fn delete(&self, db: &Database) -> Result<usize> {
		Ok(db.write()?.execute(
			r#"DELETE FROM tag_person_alt WHERE name = ?1 AND person_id = ?2"#,
			params![
				&self.name,
				&self.person_id
			]
		)?)
	}

	pub fn remove_by_id(id: PersonId, db: &Database) -> Result<usize> {
		Ok(db.write()?.execute(
			r#"DELETE FROM tag_person_alt WHERE person_id = ?1"#,
			params![id]
		)?)
	}

	pub fn transfer_or_ignore(from_id: PersonId, to_id: PersonId, db: &Database) -> Result<usize> {
		Ok(db.write()?.execute(
            r#"UPDATE OR IGNORE tag_person_alt SET person_id = ?2 WHERE person_id = ?1"#,
            params![
                from_id,
                to_id
            ]
        )?)
	}
}