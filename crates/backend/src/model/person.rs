use chrono::{DateTime, NaiveDate, NaiveDateTime, Utc};
use common::{BookId, PersonId, Source, ThumbnailStore};
use sqlx::{FromRow, SqliteConnection};

use crate::Result;
use common_local::Person;
use serde::Serialize;

#[derive(Debug)]
pub struct NewPersonModel {
    pub source: Source,

    pub name: String,
    pub description: Option<String>,
    pub birth_date: Option<NaiveDate>,

    pub thumb_url: ThumbnailStore,

    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Serialize, FromRow)]
pub struct PersonModel {
    pub id: PersonId,

    pub source: Source,

    pub name: String,
    pub description: Option<String>,

    pub birth_date: Option<NaiveDate>,

    pub thumb_url: ThumbnailStore,

    pub updated_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
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
    pub async fn insert(self, db: &mut SqliteConnection) -> Result<PersonModel> {
        let res = sqlx::query(
            "INSERT INTO tag_person (source, name, description, birth_date, thumb_url, updated_at, created_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
        )
        .bind(&self.source)
        .bind(&self.name)
        .bind(&self.description)
        .bind(self.birth_date)
        .bind(&self.thumb_url)
        .bind(self.updated_at)
        .bind(self.created_at)
        .execute(db).await?;

        Ok(PersonModel {
            id: PersonId::from(res.last_insert_rowid()),
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
    pub async fn find(offset: i64, limit: i64, db: &mut SqliteConnection) -> Result<Vec<Self>> {
        Ok(sqlx::query_as("SELECT id, source, name, description, birth_date, thumb_url, updated_at, created_at FROM tag_person LIMIT $1 OFFSET $2").bind(limit).bind(offset).fetch_all(db).await?)
    }

    pub async fn find_by_book_id(id: BookId, db: &mut SqliteConnection) -> Result<Vec<Self>> {
        Ok(sqlx::query_as(
            r#"
            SELECT id, source, name, description, birth_date, thumb_url, updated_at, created_at FROM book_person
            LEFT JOIN
                tag_person ON tag_person.id = book_person.person_id
            WHERE book_id = $1
            "#
        ).bind(id).fetch_all(db).await?)
    }

    pub async fn search_by(
        query: &str,
        offset: i64,
        limit: i64,
        db: &mut SqliteConnection,
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
            r#"SELECT id, source, name, description, birth_date, thumb_url, updated_at, created_at FROM tag_person WHERE name LIKE '%{}%' ESCAPE '{}' LIMIT $1 OFFSET $2"#,
            query
                .replace('%', &format!("{}%", escape_char))
                .replace('_', &format!("{}_", escape_char)),
            escape_char
        );

        Ok(sqlx::query_as(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await?)
    }

    pub async fn find_one_by_name(value: &str, db: &mut SqliteConnection) -> Result<Option<Self>> {
        let person = sqlx::query_as("SELECT id, source, name, description, birth_date, thumb_url, updated_at, created_at FROM tag_person WHERE name = $1").bind(value).fetch_optional(db).await?;

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

    pub async fn find_one_by_id(id: PersonId, db: &mut SqliteConnection) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, source, name, description, birth_date, thumb_url, updated_at, created_at FROM tag_person WHERE id = $1"
        ).bind(id).fetch_optional(db).await?)
    }

    pub async fn find_one_by_source(
        value: &str,
        db: &mut SqliteConnection,
    ) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, source, name, description, birth_date, thumb_url, updated_at, created_at FROM tag_person WHERE source = $1"
        ).bind(value).fetch_optional(db).await?)
    }

    pub async fn count(db: &mut SqliteConnection) -> Result<i32> {
        Ok(sqlx::query_scalar("SELECT COUNT(*) FROM tag_person")
            .fetch_one(db)
            .await?)
    }

    pub async fn update(&mut self, db: &mut SqliteConnection) -> Result<()> {
        self.updated_at = Utc::now().naive_utc();

        sqlx::query(
            r#"UPDATE tag_person SET
                source = $2,
                name = $3,
                description = $4,
                birth_date = $5,
                thumb_url = $6,
                updated_at = $7,
                created_at = $8
            WHERE id = $1"#,
        )
        .bind(self.id)
        .bind(&self.source)
        .bind(&self.name)
        .bind(&self.description)
        .bind(self.birth_date)
        .bind(&self.thumb_url)
        .bind(self.updated_at)
        .bind(self.created_at)
        .execute(db)
        .await?;

        Ok(())
    }

    pub async fn delete_by_id(id: PersonId, db: &mut SqliteConnection) -> Result<u64> {
        let res = sqlx::query("DELETE FROM tag_person WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;

        Ok(res.rows_affected())
    }
}
