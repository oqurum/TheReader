use common::PersonId;
use rusqlite::{params, OptionalExtension};

use crate::{database::Database, Result};
use serde::Serialize;

use super::{AdvRow, TableRow};

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
    pub async fn insert(&self, db: &Database) -> Result<()> {
        db.write().await.execute(
            r#"INSERT INTO tag_person_alt (name, person_id) VALUES (?1, ?2)"#,
            params![&self.name, &self.person_id],
        )?;

        Ok(())
    }

    pub async fn find_one_by_name(value: &str, db: &Database) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(
                r#"SELECT * FROM tag_person_alt WHERE name = ?1"#,
                params![value],
                |v| Self::from_row(v),
            )
            .optional()?)
    }

    pub async fn delete(&self, db: &Database) -> Result<usize> {
        Ok(db.write().await.execute(
            r#"DELETE FROM tag_person_alt WHERE name = ?1 AND person_id = ?2"#,
            params![&self.name, &self.person_id],
        )?)
    }

    pub async fn delete_by_id(id: PersonId, db: &Database) -> Result<usize> {
        Ok(db.write().await.execute(
            r#"DELETE FROM tag_person_alt WHERE person_id = ?1"#,
            params![id],
        )?)
    }

    pub async fn transfer_or_ignore(
        from_id: PersonId,
        to_id: PersonId,
        db: &Database,
    ) -> Result<usize> {
        Ok(db.write().await.execute(
            r#"UPDATE OR IGNORE tag_person_alt SET person_id = ?2 WHERE person_id = ?1"#,
            params![from_id, to_id],
        )?)
    }
}
