use common::{BookId, Either, PersonId};
use sqlx::{FromRow, SqliteConnection};

use crate::Result;
use serde::Serialize;

#[derive(Debug, Serialize, FromRow)]
pub struct BookPersonModel {
    pub book_id: BookId,
    pub person_id: PersonId,
}

impl BookPersonModel {
    pub async fn insert_or_ignore(&self, db: &mut SqliteConnection) -> Result<()> {
        sqlx::query(
            "INSERT OR IGNORE INTO book_person (book_id, person_id) VALUES ($1, $2)"
        ).bind(self.book_id).bind(self.person_id).execute(db).await?;

        Ok(())
    }

    pub async fn delete(&self, db: &mut SqliteConnection) -> Result<()> {
        sqlx::query(
            "DELETE FROM book_person WHERE book_id = $1 AND person_id = $2"
        ).bind(self.book_id).bind(self.person_id).execute(db).await?;

        Ok(())
    }

    pub async fn delete_by_book_id(id: BookId, db: &mut SqliteConnection) -> Result<()> {
        sqlx::query(
            "DELETE FROM book_person WHERE book_id = $1"
        ).bind(id).execute(db).await?;

        Ok(())
    }

    pub async fn delete_by_person_id(id: PersonId, db: &mut SqliteConnection) -> Result<()> {
        sqlx::query(
            "DELETE FROM book_person WHERE person_id = $1"
        ).bind(id).execute(db).await?;

        Ok(())
    }

    pub async fn transfer_person(
        from_id: PersonId,
        to_id: PersonId,
        db: &mut SqliteConnection,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE book_person SET person_id = $2 WHERE person_id = $1"
        ).bind(from_id).bind(to_id).execute(db).await?;

        Ok(res.rows_affected())
    }

    pub async fn find_by(
        id: Either<BookId, PersonId>,
        db: &mut SqliteConnection,
    ) -> Result<Vec<Self>> {
        match id {
            Either::Left(id) => {
                Ok(sqlx::query_as("SELECT * FROM book_person WHERE book_id = $1").bind(id).fetch_all(db).await?)
            }

            Either::Right(id) => {
                Ok(sqlx::query_as("SELECT * FROM book_person WHERE person_id = $1").bind(id).fetch_all(db).await?)
            }
        }
    }
}
