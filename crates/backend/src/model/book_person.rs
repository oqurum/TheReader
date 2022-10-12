use common::{BookId, Either, PersonId};
use rusqlite::params;

use crate::{database::Database, Result};
use serde::Serialize;

use super::{AdvRow, TableRow};

#[derive(Debug, Serialize)]
pub struct BookPersonModel {
    pub book_id: BookId,
    pub person_id: PersonId,
}

impl TableRow<'_> for BookPersonModel {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            book_id: row.next()?,
            person_id: row.next()?,
        })
    }
}

impl BookPersonModel {
    pub async fn insert_or_ignore(&self, db: &Database) -> Result<()> {
        db.write().await.execute(
            r#"INSERT OR IGNORE INTO book_person (book_id, person_id) VALUES (?1, ?2)"#,
            params![self.book_id, self.person_id],
        )?;

        Ok(())
    }

    pub async fn delete(&self, db: &Database) -> Result<()> {
        db.write().await.execute(
            r#"DELETE FROM book_person WHERE book_id = ?1 AND person_id = ?2"#,
            params![self.book_id, self.person_id],
        )?;

        Ok(())
    }

    pub async fn delete_by_book_id(id: BookId, db: &Database) -> Result<()> {
        db.write()
            .await
            .execute(r#"DELETE FROM book_person WHERE book_id = ?1"#, [id])?;

        Ok(())
    }

    pub async fn delete_by_person_id(id: PersonId, db: &Database) -> Result<()> {
        db.write()
            .await
            .execute(r#"DELETE FROM book_person WHERE person_id = ?1"#, [id])?;

        Ok(())
    }

    pub async fn transfer_person(
        from_id: PersonId,
        to_id: PersonId,
        db: &Database,
    ) -> Result<usize> {
        Ok(db.write().await.execute(
            r#"UPDATE book_person SET person_id = ?2 WHERE person_id = ?1"#,
            [from_id, to_id],
        )?)
    }

    pub async fn find_by(id: Either<BookId, PersonId>, db: &Database) -> Result<Vec<Self>> {
        let this = db.read().await;

        match id {
            Either::Left(id) => {
                let mut conn = this.prepare(r#"SELECT * FROM book_person WHERE book_id = ?1"#)?;

                let map = conn.query_map([id], |v| Self::from_row(v))?;

                Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
            }

            Either::Right(id) => {
                let mut conn = this.prepare(r#"SELECT * FROM book_person WHERE person_id = ?1"#)?;

                let map = conn.query_map([id], |v| Self::from_row(v))?;

                Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
            }
        }
    }
}
