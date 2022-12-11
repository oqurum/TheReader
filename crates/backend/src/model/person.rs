use chrono::{DateTime, NaiveDate, Utc};
use common::{BookId, PersonId, Source, ThumbnailStore};
use rusqlite::{params, OptionalExtension};

use crate::{DatabaseAccess, Result};
use common_local::Person;
use serde::Serialize;

use super::{AdvRow, TableRow};

#[derive(Debug)]
pub struct NewPersonModel {
    pub source: Source,

    pub name: String,
    pub description: Option<String>,
    pub birth_date: Option<NaiveDate>,

    pub thumb_url: ThumbnailStore,

    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PersonModel {
    pub id: PersonId,

    pub source: Source,

    pub name: String,
    pub description: Option<String>,

    pub birth_date: Option<NaiveDate>,

    pub thumb_url: ThumbnailStore,

    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl TableRow<'_> for PersonModel {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.next()?,

            source: Source::try_from(row.next::<String>()?).unwrap(),

            name: row.next()?,
            description: row.next()?,
            birth_date: row.next()?,

            thumb_url: ThumbnailStore::from(row.next_opt::<String>()?),

            updated_at: row.next()?,
            created_at: row.next()?,
        })
    }
}

impl From<PersonModel> for Person {
    fn from(val: PersonModel) -> Self {
        Person {
            id: val.id,
            source: val.source,
            name: val.name,
            description: val.description,
            birth_date: val.birth_date,
            thumb_url: val.thumb_url,
            updated_at: val.updated_at,
            created_at: val.created_at,
        }
    }
}

impl NewPersonModel {
    pub async fn insert(self, db: &dyn DatabaseAccess) -> Result<PersonModel> {
        let conn = db.write().await;

        conn.execute(r#"
            INSERT INTO tag_person (source, name, description, birth_date, thumb_url, updated_at, created_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)
        "#,
        params![
            self.source.to_string(), &self.name, &self.description, &self.birth_date, self.thumb_url.as_value(),
            self.updated_at, self.created_at
        ])?;

        Ok(PersonModel {
            id: PersonId::from(conn.last_insert_rowid() as usize),
            source: self.source,
            name: self.name,
            description: self.description,
            birth_date: self.birth_date,
            thumb_url: self.thumb_url,
            updated_at: self.updated_at,
            created_at: self.created_at,
        })
    }
}

impl PersonModel {
    pub async fn find(offset: usize, limit: usize, db: &dyn DatabaseAccess) -> Result<Vec<Self>> {
        let this = db.read().await;

        let mut conn = this.prepare(r#"SELECT * FROM tag_person LIMIT ?1 OFFSET ?2"#)?;

        let map = conn.query_map([limit, offset], |v| Self::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn find_by_book_id(id: BookId, db: &dyn DatabaseAccess) -> Result<Vec<Self>> {
        let this = db.read().await;

        let mut conn = this.prepare(
            r#"
            SELECT tag_person.* FROM book_person
            LEFT JOIN
                tag_person ON tag_person.id = book_person.person_id
            WHERE book_id = ?1
        "#,
        )?;

        let map = conn.query_map([id], |v| Self::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn search_by(
        query: &str,
        offset: usize,
        limit: usize,
        db: &dyn DatabaseAccess,
    ) -> Result<Vec<Self>> {
        let mut escape_char = '\\';
        // Change our escape character if it's in the query.
        if query.contains(escape_char) {
            for car in [
                '!', '@', '#', '$', '^', '&', '*', '-', '=', '+', '|', '~', '`', '/', '?', '>',
                '<', ',',
            ] {
                if !query.contains(car) {
                    escape_char = car;
                    break;
                }
            }
        }

        let sql = format!(
            r#"SELECT * FROM tag_person WHERE name LIKE '%{}%' ESCAPE '{}' LIMIT ?1 OFFSET ?2"#,
            query
                .replace('%', &format!("{}%", escape_char))
                .replace('_', &format!("{}_", escape_char)),
            escape_char
        );

        let this = db.read().await;

        let mut conn = this.prepare(&sql)?;

        let map = conn.query_map(params![limit, offset], |v| Self::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn find_one_by_name(value: &str, db: &dyn DatabaseAccess) -> Result<Option<Self>> {
        let person = db
            .read()
            .await
            .query_row(
                r#"SELECT * FROM tag_person WHERE name = ?1"#,
                params![value],
                |v| Self::from_row(v),
            )
            .optional()?;

        if let Some(person) = person {
            Ok(Some(person))
        } else {
            Ok(None)
        }
        // TODO: Enable at a later date?
        // else if let Some(alt) = PersonAltModel::find_one_by_name(value, db).await? {
        //     Self::find_one_by_id(alt.person_id, db).await
        // }
    }

    pub async fn find_one_by_id(id: PersonId, db: &dyn DatabaseAccess) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(
                r#"SELECT * FROM tag_person WHERE id = ?1"#,
                params![id],
                |v| Self::from_row(v),
            )
            .optional()?)
    }

    pub async fn find_one_by_source(value: &str, db: &dyn DatabaseAccess) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(
                r#"SELECT * FROM tag_person WHERE source = ?1"#,
                params![value],
                |v| Self::from_row(v),
            )
            .optional()?)
    }

    pub async fn count(db: &dyn DatabaseAccess) -> Result<usize> {
        Ok(db
            .read()
            .await
            .query_row(r#"SELECT COUNT(*) FROM tag_person"#, [], |v| v.get(0))?)
    }

    pub async fn update(&self, db: &dyn DatabaseAccess) -> Result<()> {
        db.write().await.execute(
            r#"
            UPDATE tag_person SET
                source = ?2,
                name = ?3,
                description = ?4,
                birth_date = ?5,
                thumb_url = ?6,
                updated_at = ?7,
                created_at = ?8
            WHERE id = ?1"#,
            params![
                self.id,
                self.source.to_string(),
                &self.name,
                &self.description,
                &self.birth_date,
                self.thumb_url.as_value(),
                self.updated_at,
                self.created_at
            ],
        )?;

        Ok(())
    }

    pub async fn delete_by_id(id: PersonId, db: &dyn DatabaseAccess) -> Result<usize> {
        Ok(db
            .write()
            .await
            .execute(r#"DELETE FROM tag_person WHERE id = ?1"#, params![id])?)
    }
}
