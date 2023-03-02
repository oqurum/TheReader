use chrono::{DateTime, Utc};
use common::{BookId, PersonId, Source, ThumbnailStore};
use rusqlite::{params, OptionalExtension};

use crate::{DatabaseAccess, Result, config::get_config};
use common_local::{
    filter::{FilterContainer, FilterModifier, FilterTableType},
    BookEdit, BookItemCached, DisplayBookItem, LibraryId, BookType,
};
use serde::Serialize;

use super::{book_person::BookPersonModel, AdvRow, TableRow};

#[derive(Debug, Clone, Serialize)]
pub struct BookModel {
    pub id: BookId,

    pub library_id: LibraryId,

    pub type_of: BookType,

    pub source: Source,
    pub file_item_count: i64,
    pub title: Option<String>,
    pub original_title: Option<String>,
    pub description: Option<String>,
    pub rating: f64,

    pub thumb_path: ThumbnailStore,
    pub all_thumb_urls: Vec<String>,

    // TODO: Make table for all tags. Include publisher in it. Remove country.
    pub cached: BookItemCached,

    pub refreshed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,

    pub available_at: Option<DateTime<Utc>>,
    pub year: Option<i64>,
}

impl From<BookModel> for DisplayBookItem {
    fn from(val: BookModel) -> Self {
        /// Get the public user-accessible url.
        fn get_public_url(source: &Source, libby_url: Option<String>) -> Option<String> {
            match source.agent.as_ref() {
                "libby" => Some(format!("{}/book/{}", libby_url?, source.value)),
                "googlebooks" => Some(format!("https://books.google.com/books?id={}", source.value)),
                "openlibrary" => Some(format!("https://openlibrary.org/isbn/{}", source.value)),

                _ => None,
            }
        }

        DisplayBookItem {
            id: val.id,
            library_id: val.library_id,
            type_of: val.type_of,
            public_source_url: get_public_url(
                &val.source,
                Some(get_config().libby.url)
                    .filter(|v| !v.is_empty())
                    .map(|v| v.trim().to_string())
            ),
            source: val.source,
            file_item_count: val.file_item_count,
            title: val.title,
            original_title: val.original_title,
            description: val.description,
            rating: val.rating,
            thumb_path: val.thumb_path,
            cached: val.cached,
            refreshed_at: val.refreshed_at,
            created_at: val.created_at,
            updated_at: val.updated_at,
            deleted_at: val.deleted_at,
            available_at: val.available_at.map(|v| v.timestamp_millis()),
            year: val.year,
        }
    }
}

impl TableRow<'_> for BookModel {
    fn create(row: &mut AdvRow<'_>) -> rusqlite::Result<Self> {
        Ok(Self {
            id: row.next()?,
            library_id: row.next()?,
            type_of: BookType::try_from(row.next::<i32>()?).unwrap(),
            source: Source::try_from(row.next::<String>()?).unwrap(),
            file_item_count: row.next()?,
            title: row.next()?,
            original_title: row.next()?,
            description: row.next()?,
            rating: row.next()?,
            thumb_path: ThumbnailStore::from(row.next_opt::<String>()?),
            all_thumb_urls: Vec::new(),
            cached: row
                .next_opt::<String>()?
                .map(BookItemCached::from_string)
                .unwrap_or_default(),
            available_at: row.next()?,
            year: row.next()?,
            refreshed_at: row.next()?,
            created_at: row.next()?,
            updated_at: row.next()?,
            deleted_at: row.next_opt()?,
        })
    }
}

impl BookModel {
    pub async fn insert_or_increment(&self, db: &dyn DatabaseAccess) -> Result<Self> {
        let table_book = if self.id != 0 {
            Self::find_one_by_id(self.id, db).await?
        } else {
            Self::find_one_by_source(&self.source, db).await?
        };

        if table_book.is_none() {
            db.write().await.execute(
                r#"
                INSERT INTO book (
                    library_id, type_of, source, file_item_count,
                    title, original_title, description, rating, thumb_url,
                    cached,
                    available_at, year,
                    refreshed_at, created_at, updated_at, deleted_at
                )
                VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)"#,
                params![
                    self.library_id,
                    i32::from(self.type_of),
                    self.source.to_string(),
                    &self.file_item_count,
                    &self.title,
                    &self.original_title,
                    &self.description,
                    &self.rating,
                    self.thumb_path.as_value(),
                    &self.cached.as_string_optional(),
                    &self.available_at,
                    &self.year,
                    self.refreshed_at,
                    self.created_at,
                    self.updated_at,
                    self.deleted_at,
                ],
            )?;

            return Ok(Self::find_one_by_source(&self.source, db).await?.unwrap());
        } else if self.id != 0 {
            db.write().await.execute(
                r#"UPDATE book SET file_item_count = file_item_count + 1 WHERE id = ?1"#,
                params![self.id],
            )?;
        } else {
            db.write().await.execute(
                r#"UPDATE book SET file_item_count = file_item_count + 1 WHERE source = ?1"#,
                params![self.source.to_string()],
            )?;
        }

        Ok(table_book.unwrap())
    }

    pub async fn update(&mut self, db: &dyn DatabaseAccess) -> Result<()> {
        self.updated_at = Utc::now();

        db.write().await.execute(
            r#"
            UPDATE book SET
                library_id = ?2, source = ?3, file_item_count = ?4,
                title = ?5, original_title = ?6, description = ?7, rating = ?8, thumb_url = ?9,
                cached = ?10,
                available_at = ?11, year = ?12,
                refreshed_at = ?13, updated_at = ?14, deleted_at = ?15, type_of = ?16
            WHERE id = ?1"#,
            params![
                self.id,
                self.library_id,
                self.source.to_string(),
                &self.file_item_count,
                &self.title,
                &self.original_title,
                &self.description,
                &self.rating,
                self.thumb_path.as_value(),
                &self.cached.as_string_optional(),
                &self.available_at,
                &self.year,
                self.refreshed_at,
                self.updated_at,
                self.deleted_at,
                i32::from(self.type_of),
            ],
        )?;

        Ok(())
    }

    pub async fn delete_or_decrement(id: BookId, db: &dyn DatabaseAccess) -> Result<()> {
        if let Some(model) = Self::find_one_by_id(id, db).await? {
            if model.file_item_count < 1 {
                db.write().await.execute(
                    r#"UPDATE book SET file_item_count = file_item_count - 1 WHERE id = ?1"#,
                    params![id],
                )?;
            } else {
                db.write()
                    .await
                    .execute(r#"DELETE FROM book WHERE id = ?1"#, params![id])?;
            }
        }

        Ok(())
    }

    pub async fn decrement(id: BookId, db: &dyn DatabaseAccess) -> Result<()> {
        if let Some(model) = Self::find_one_by_id(id, db).await? {
            if model.file_item_count > 0 {
                db.write().await.execute(
                    r#"UPDATE book SET file_item_count = file_item_count - 1 WHERE id = ?1"#,
                    params![id],
                )?;
            }
        }

        Ok(())
    }

    pub async fn set_file_count(
        id: BookId,
        file_count: usize,
        db: &dyn DatabaseAccess,
    ) -> Result<()> {
        db.write().await.execute(
            r#"UPDATE book SET file_item_count = ?2 WHERE id = ?1"#,
            params![id, file_count],
        )?;

        Ok(())
    }

    // TODO: Change to get_metadata_by_hash. We shouldn't get metadata by source. Local metadata could be different with the same source id.
    pub async fn find_one_by_source(
        source: &Source,
        db: &dyn DatabaseAccess,
    ) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(
                r#"SELECT * FROM book WHERE source = ?1"#,
                params![source.to_string()],
                |v| BookModel::from_row(v),
            )
            .optional()?)
    }

    pub async fn find_one_by_id(id: BookId, db: &dyn DatabaseAccess) -> Result<Option<Self>> {
        Ok(db
            .read()
            .await
            .query_row(r#"SELECT * FROM book WHERE id = ?1"#, params![id], |v| {
                BookModel::from_row(v)
            })
            .optional()?)
    }

    pub async fn delete_by_id(id: BookId, db: &dyn DatabaseAccess) -> Result<usize> {
        Ok(db
            .write()
            .await
            .execute(r#"DELETE FROM book WHERE id = ?1"#, params![id])?)
    }

    pub async fn find_by(
        library: Option<LibraryId>,
        offset: usize,
        limit: usize,
        person_id: Option<PersonId>,
        db: &dyn DatabaseAccess,
    ) -> Result<Vec<Self>> {
        let this = db.read().await;

        let insert_where = (library.is_some() || person_id.is_some())
            .then_some("WHERE")
            .unwrap_or_default();
        let insert_and = (library.is_some() && person_id.is_some())
            .then_some("AND")
            .unwrap_or_default();

        let lib_id = library
            .map(|v| format!("library_id={v}"))
            .unwrap_or_default();

        let inner_query = person_id
            .map(|pid| {
                format!(r#"id IN (SELECT book_id FROM book_person WHERE person_id = {pid})"#)
            })
            .unwrap_or_default();

        let mut conn = this.prepare(&format!(r#"SELECT * FROM book {insert_where} {lib_id} {insert_and} {inner_query} LIMIT ?1 OFFSET ?2"#))?;

        let map = conn.query_map([limit, offset], |v| BookModel::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn edit_book_by_id(
        book_id: BookId,
        edit: BookEdit,
        db: &dyn DatabaseAccess,
    ) -> Result<usize> {
        let mut items = Vec::new();

        let mut values = vec![&book_id as &dyn rusqlite::ToSql];

        if let Some(value) = edit.title.as_ref() {
            items.push("title");
            values.push(value as &dyn rusqlite::ToSql);
        }

        if let Some(value) = edit.original_title.as_ref() {
            items.push("original_title");
            values.push(value as &dyn rusqlite::ToSql);
        }

        if let Some(value) = edit.description.as_ref() {
            items.push("description");
            values.push(value as &dyn rusqlite::ToSql);
        }

        if let Some(value) = edit.rating.as_ref() {
            items.push("rating");
            values.push(value as &dyn rusqlite::ToSql);
        }

        if let Some(value) = edit.available_at.as_ref() {
            items.push("available_at");
            values.push(value as &dyn rusqlite::ToSql);
        }

        if let Some(value) = edit.year.as_ref() {
            items.push("year");
            values.push(value as &dyn rusqlite::ToSql);
        }

        if let Some(_value) = edit.publisher {
            // TODO
        }

        if let Some(ids) = edit.added_people {
            for person_id in ids {
                BookPersonModel { book_id, person_id }
                    .insert_or_ignore(db)
                    .await?;
            }
        }

        if let Some(ids) = edit.removed_people {
            for person_id in ids {
                BookPersonModel { book_id, person_id }.delete(db).await?;
            }
        }

        if items.is_empty() {
            return Ok(0);
        }

        Ok(db.write().await.execute(
            &format!(
                "UPDATE book SET {} WHERE id = ?1",
                items
                    .iter()
                    .enumerate()
                    .map(|(i, v)| format!("{v} = ?{}", 2 + i))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            rusqlite::params_from_iter(values.iter()),
        )?)
    }

    // Search
    fn gen_search_query(filter: &FilterContainer, library: Option<LibraryId>) -> String {
        let mut sql = String::from("SELECT * FROM book WHERE ");
        let orig_len = sql.len();

        let mut f_comp = Vec::new();

        // Library ID
        if let Some(library) = library {
            f_comp.push(format!("library_id={library} "));
        }

        for fil in &filter.filters {
            match fil.type_of {
                FilterTableType::Id => todo!(),

                FilterTableType::CreatedAt => todo!(),

                FilterTableType::Source => {
                    for query in fil.value.values() {
                        f_comp.push(format!(
                            "source {} '{}%' ",
                            get_modifier(fil.type_of, fil.modifier),
                            query
                        ));
                    }
                }

                FilterTableType::Query => {
                    for query in fil.value.values() {
                        let mut escape_char = '\\';
                        // Change our escape character if it's in the query.
                        if query.contains(escape_char) {
                            for car in [
                                '!', '@', '#', '$', '^', '&', '*', '-', '=', '+', '|', '~', '`',
                                '/', '?', '>', '<', ',',
                            ] {
                                if !query.contains(car) {
                                    escape_char = car;
                                    break;
                                }
                            }
                        }

                        // TODO: Utilize title > original_title > description, and sort
                        f_comp.push(format!(
                            "title {} '%{}%' ESCAPE '{}' ",
                            get_modifier(fil.type_of, fil.modifier),
                            query
                                .replace('%', &format!("{escape_char}%"))
                                .replace('_', &format!("{escape_char}_")),
                            escape_char
                        ));
                    }
                }

                FilterTableType::Person => {
                    for pid in fil.value.values() {
                        match fil.modifier {
                            FilterModifier::IsNull => {
                                f_comp.push(String::from(
                                    "id NOT IN (SELECT book_id FROM book_person WHERE book_id = book.id)"
                                ));
                            }

                            FilterModifier::IsNotNull => {
                                f_comp.push(String::from(
                                    "id IN (SELECT book_id FROM book_person WHERE book_id = book.id)"
                                ));
                            }

                            v => {
                                f_comp.push(format!(
                                    "id IN (SELECT book_id FROM book_person WHERE person_id {} {})",
                                    get_modifier(fil.type_of, v),
                                    pid
                                ));
                            }
                        }
                    }
                }
            }
        }

        sql += &f_comp.join(" AND ");

        if let Some((order_name, is_desc)) = filter.order_by {
            let field_name = match order_name {
                FilterTableType::Id => "id",
                FilterTableType::Query => "title",
                FilterTableType::CreatedAt => "created_at",
                FilterTableType::Source => todo!(),
                FilterTableType::Person => todo!(),
            };

            sql += &format!(
                " ORDER BY {field_name} {} ",
                if is_desc { "DESC" } else { "ASC" }
            );
        }

        if sql.len() == orig_len {
            String::from("SELECT * FROM book ")
        } else {
            sql
        }
    }

    pub async fn search_by(
        filter: &FilterContainer,
        library: Option<LibraryId>,
        offset: usize,
        limit: usize,
        db: &dyn DatabaseAccess,
    ) -> Result<Vec<Self>> {
        let mut sql = Self::gen_search_query(filter, library);

        sql += "LIMIT ?1 OFFSET ?2";

        let this = db.read().await;

        let mut conn = this.prepare(&sql)?;

        let map = conn.query_map([limit, offset], |v| BookModel::from_row(v))?;

        Ok(map.collect::<std::result::Result<Vec<_>, _>>()?)
    }

    pub async fn count_search_by(
        filter: &FilterContainer,
        library: Option<LibraryId>,
        db: &dyn DatabaseAccess,
    ) -> Result<usize> {
        let sql = Self::gen_search_query(filter, library).replace("SELECT *", "SELECT COUNT(*)");

        Ok(db.read().await.query_row(&sql, [], |v| v.get(0))?)
    }
}

fn get_modifier(type_of: FilterTableType, modi: FilterModifier) -> &'static str {
    match (type_of, modi) {
        (FilterTableType::Source, FilterModifier::Equal)
        | (FilterTableType::Query, FilterModifier::Equal) => "LIKE",

        (_, FilterModifier::IsNull) => "IS NULL",
        (_, FilterModifier::IsNotNull) => "IS NOT NULL",
        (_, FilterModifier::GreaterThan) => ">",
        (_, FilterModifier::GreaterThanOrEqual) => ">=",
        (_, FilterModifier::LessThan) => "<",
        (_, FilterModifier::LessThanOrEqual) => "<=",
        (_, FilterModifier::Equal) => "=",
        (_, FilterModifier::DoesNotEqual) => "!=",
    }
}
