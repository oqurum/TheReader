use common::PersonId;
use sqlx::{FromRow, SqliteConnection};

use crate::Result;
use serde::Serialize;

#[derive(Debug, Serialize, FromRow)]
pub struct PersonAltModel {
    pub person_id: PersonId,
    pub name: String,
}

impl PersonAltModel {
    pub async fn insert(&self, db: &mut SqliteConnection) -> Result<()> {
        sqlx::query("INSERT INTO tag_person_alt (name, person_id) VALUES ($1, $2)")
            .bind(&self.name)
            .bind(self.person_id)
            .execute(db)
            .await?;

        Ok(())
    }

    pub async fn find_one_by_name(value: &str, db: &mut SqliteConnection) -> Result<Option<Self>> {
        Ok(
            sqlx::query_as("SELECT * FROM tag_person_alt WHERE name = $1")
                .bind(value)
                .fetch_optional(db)
                .await?,
        )
    }

    pub async fn delete(&self, db: &mut SqliteConnection) -> Result<u64> {
        let res = sqlx::query("DELETE FROM tag_person_alt WHERE name = $1 AND person_id = $2")
            .bind(&self.name)
            .bind(self.person_id)
            .execute(db)
            .await?;

        Ok(res.rows_affected())
    }

    pub async fn delete_by_id(id: PersonId, db: &mut SqliteConnection) -> Result<u64> {
        let res = sqlx::query("DELETE FROM tag_person_alt WHERE person_id = $1")
            .bind(id)
            .execute(db)
            .await?;

        Ok(res.rows_affected())
    }

    pub async fn transfer_or_ignore(
        from_id: PersonId,
        to_id: PersonId,
        db: &mut SqliteConnection,
    ) -> Result<u64> {
        let res =
            sqlx::query("UPDATE OR IGNORE tag_person_alt SET person_id = $2 WHERE person_id = $1")
                .bind(from_id)
                .bind(to_id)
                .execute(db)
                .await?;

        Ok(res.rows_affected())
    }
}
