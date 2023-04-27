use chrono::{NaiveDateTime, Utc};
use common::{BookId, PersonId, Source, ThumbnailStore};
use sqlx::{FromRow, SqliteConnection};

use crate::{config::get_config, Result};
use common_local::{
    filter::{FilterContainer, FilterModifier, FilterTableType},
    BookEdit, BookItemCached, BookType, DisplayBookItem, LibraryId,
};
use serde::Serialize;

use super::book_person::BookPersonModel;

#[derive(Debug, Clone, Serialize)]
pub struct NewBookModel {
    pub library_id: LibraryId,

    pub type_of: BookType,
    pub parent_id: Option<BookId>,

    pub source: Source,
    pub file_item_count: i64,
    pub title: Option<String>,
    pub original_title: Option<String>,
    pub description: Option<String>,
    pub rating: f64,

    pub thumb_url: ThumbnailStore,
    // pub all_thumb_urls: Vec<String>,

    // TODO: Make table for all tags. Include publisher in it. Remove country.
    pub cached: BookItemCached,
    pub index: Option<i64>,

    pub refreshed_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,

    pub available_at: Option<NaiveDateTime>,
    pub year: Option<i64>,
}

#[derive(Debug, Clone, Serialize, FromRow)]
pub struct BookModel {
    pub id: BookId,

    pub library_id: LibraryId,

    pub type_of: BookType,
    pub parent_id: Option<BookId>,

    pub source: Source,
    pub file_item_count: i64,
    pub title: Option<String>,
    pub original_title: Option<String>,
    pub description: Option<String>,
    pub rating: f64,

    pub thumb_url: ThumbnailStore,
    // pub all_thumb_urls: Vec<String>,

    // TODO: Make table for all tags. Include publisher in it. Remove country.
    pub cached: BookItemCached,
    pub index: Option<i64>,

    pub refreshed_at: NaiveDateTime,
    pub created_at: NaiveDateTime,
    pub updated_at: NaiveDateTime,
    pub deleted_at: Option<NaiveDateTime>,

    pub available_at: Option<NaiveDateTime>,
    pub year: Option<i64>,
}

impl From<BookModel> for DisplayBookItem {
    fn from(val: BookModel) -> Self {
        /// Get the public user-accessible url.
        fn get_public_url(source: &Source, libby_url: Option<String>) -> Option<String> {
            match source.agent.as_ref() {
                "libby" => Some(format!("{}/book/{}", libby_url?, source.value)),
                "googlebooks" => Some(format!(
                    "https://books.google.com/books?id={}",
                    source.value
                )),
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
                    .map(|v| v.trim().to_string()),
            ),
            source: val.source,
            file_item_count: val.file_item_count,
            title: val.title,
            original_title: val.original_title,
            description: val.description,
            rating: val.rating,
            thumb_path: val.thumb_url,
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

impl NewBookModel {
    pub fn new_section(
        is_prologue: bool,
        library_id: LibraryId,
        parent_id: BookId,
        source: Source,
    ) -> Self {
        let now = Utc::now().naive_utc();

        Self {
            library_id,
            type_of: BookType::ComicBookSection,
            parent_id: Some(parent_id),
            source,
            file_item_count: 0,
            title: None,
            original_title: None,
            description: None,
            rating: 0.0,
            thumb_url: ThumbnailStore::None,
            cached: BookItemCached::default(),
            index: Some(if is_prologue { 0 } else { 1 }),
            refreshed_at: now,
            created_at: now,
            updated_at: now,
            deleted_at: None,
            available_at: None,
            year: None,
        }
    }

    pub async fn insert(self, db: &mut SqliteConnection) -> Result<BookModel> {
        let res = sqlx::query(
            r#"INSERT INTO book (
                library_id, type_of, parent_id, source, file_item_count,
                title, original_title, description, rating, thumb_url,
                cached, "index",
                available_at, year,
                refreshed_at, created_at, updated_at, deleted_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18)"#
        )
        .bind(self.library_id)
        .bind(self.type_of)
        .bind(self.parent_id)
        .bind(self.source.to_string())
        .bind(self.file_item_count)
        .bind(&self.title)
        .bind(&self.original_title)
        .bind(&self.description)
        .bind(self.rating)
        .bind(self.thumb_url.as_value())
        .bind(self.cached.as_string_optional())
        .bind(self.index)
        .bind(self.available_at)
        .bind(self.year)
        .bind(self.refreshed_at)
        .bind(self.created_at)
        .bind(self.updated_at)
        .bind(self.deleted_at)
        .execute(db).await?;

        Ok(self.set_id(BookId::from(res.last_insert_rowid())))
    }

    pub async fn insert_or_increment(self, db: &mut SqliteConnection) -> Result<BookModel> {
        if let Some(mut table_book) = BookModel::find_one_by_source(&self.source, db).await? {
            sqlx::query("UPDATE book SET file_item_count = file_item_count + 1 WHERE source = $1")
                .bind(&self.source)
                .execute(db)
                .await?;

            table_book.file_item_count += 1;

            Ok(table_book)
        } else {
            self.insert(db).await
        }
    }

    pub fn set_id(self, id: BookId) -> BookModel {
        BookModel {
            id,
            library_id: self.library_id,
            type_of: self.type_of,
            parent_id: self.parent_id,
            source: self.source,
            file_item_count: self.file_item_count,
            title: self.title,
            original_title: self.original_title,
            description: self.description,
            rating: self.rating,
            thumb_url: self.thumb_url,
            // all_thumb_urls: self.all_thumb_urls,
            cached: self.cached,
            index: self.index,
            available_at: self.available_at,
            year: self.year,
            refreshed_at: self.refreshed_at,
            created_at: self.created_at,
            updated_at: self.updated_at,
            deleted_at: self.deleted_at,
        }
    }
}

impl BookModel {
    pub async fn update(&mut self, db: &mut SqliteConnection) -> Result<()> {
        self.updated_at = Utc::now().naive_utc();

        sqlx::query(
            r#"UPDATE book SET
                library_id = $2, source = $3, file_item_count = $4,
                title = $5, original_title = $6, description = $7, rating = $8, thumb_url = $9,
                cached = $10,
                available_at = $11, year = $12,
                refreshed_at = $13, updated_at = $14, deleted_at = $15, type_of = $16, parent_id = $17, "index" = $18
            WHERE id = $1"#
        )
        .bind(self.id)
        .bind(self.library_id)
        .bind(self.source.to_string())
        .bind(self.file_item_count)
        .bind(&self.title)
        .bind(&self.original_title)
        .bind(&self.description)
        .bind(self.rating)
        .bind(self.thumb_url.as_value())
        .bind(self.cached.as_string_optional())
        .bind(self.available_at)
        .bind(self.year)
        .bind(self.refreshed_at)
        .bind(self.updated_at)
        .bind(self.deleted_at)
        .bind(self.type_of)
        .bind(self.parent_id)
        .bind(self.index)
        .execute(db).await?;

        Ok(())
    }

    pub async fn increment(id: BookId, db: &mut SqliteConnection) -> Result<()> {
        sqlx::query("UPDATE book SET file_item_count = file_item_count + 1 WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;

        Ok(())
    }

    pub async fn delete_or_decrement(id: BookId, db: &mut SqliteConnection) -> Result<()> {
        if let Some(model) = Self::find_one_by_id(id, db).await? {
            if model.file_item_count < 1 {
                sqlx::query("UPDATE book SET file_item_count = file_item_count - 1 WHERE id = $1")
                    .bind(id)
                    .execute(db)
                    .await?;
            } else {
                sqlx::query("DELETE FROM book WHERE id = $1")
                    .bind(id)
                    .execute(db)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn decrement(id: BookId, db: &mut SqliteConnection) -> Result<()> {
        if let Some(model) = Self::find_one_by_id(id, db).await? {
            if model.file_item_count > 0 {
                sqlx::query("UPDATE book SET file_item_count = file_item_count - 1 WHERE id = $1")
                    .bind(id)
                    .execute(db)
                    .await?;
            }
        }

        Ok(())
    }

    pub async fn set_file_count(
        id: BookId,
        file_count: i64,
        db: &mut SqliteConnection,
    ) -> Result<()> {
        sqlx::query("UPDATE book SET file_item_count = $2 WHERE id = $1")
            .bind(id)
            .bind(file_count)
            .execute(db)
            .await?;

        Ok(())
    }

    // TODO: Change to get_metadata_by_hash. We shouldn't get metadata by source. Local metadata could be different with the same source id.
    pub async fn find_one_by_source(
        source: &Source,
        db: &mut SqliteConnection,
    ) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, library_id, type_of, parent_id, source, file_item_count, title, original_title, description, rating, thumb_url, cached, \"index\", refreshed_at, created_at, updated_at, deleted_at, available_at, year FROM book WHERE source = $1 AND (type_of = $2 OR type_of = $3)"
        ).bind(source).bind(BookType::Book).bind(BookType::ComicBook).fetch_optional(db).await?)
    }

    pub async fn find_one_by_id(id: BookId, db: &mut SqliteConnection) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, library_id, type_of, parent_id, source, file_item_count, title, original_title, description, rating, thumb_url, cached, \"index\", refreshed_at, created_at, updated_at, deleted_at, available_at, year FROM book WHERE id = $1"
        ).bind(id).fetch_optional(db).await?)
    }

    pub async fn find_by_parent_id(id: BookId, db: &mut SqliteConnection) -> Result<Vec<Self>> {
        Ok(sqlx::query_as(
            "SELECT id, library_id, type_of, parent_id, source, file_item_count, title, original_title, description, rating, thumb_url, cached, \"index\", refreshed_at, created_at, updated_at, deleted_at, available_at, year FROM book WHERE parent_id = $1"
        ).bind(id).fetch_all(db).await?)
    }

    pub async fn find_one_by_parent_id_and_index(
        id: BookId,
        index: i64,
        db: &mut SqliteConnection,
    ) -> Result<Option<Self>> {
        Ok(sqlx::query_as(
                "SELECT id, library_id, type_of, parent_id, source, file_item_count, title, original_title, description, rating, thumb_url, cached, \"index\", refreshed_at, created_at, updated_at, deleted_at, available_at, year FROM book WHERE parent_id = $1 AND \"index\" = $2"
            ).bind(id).bind(index).fetch_optional(db).await?)
    }

    pub async fn delete_by_id(id: BookId, db: &mut SqliteConnection) -> Result<u64> {
        let res = sqlx::query("DELETE FROM book WHERE id = $1")
            .bind(id)
            .execute(db)
            .await?;

        Ok(res.rows_affected())
    }

    pub async fn find_by(
        library: Option<LibraryId>,
        offset: i64,
        limit: i64,
        person_id: Option<PersonId>,
        db: &mut SqliteConnection,
    ) -> Result<Vec<Self>> {
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

        let sql = format!(
            r#"SELECT * FROM book {insert_where} {lib_id} {insert_and} {inner_query} LIMIT $1 OFFSET $2"#
        );

        let conn = sqlx::query_as(&sql);

        Ok(conn.bind(limit).bind(offset).fetch_all(db).await?)
    }

    pub async fn edit_book_by_id(
        book_id: BookId,
        edit: BookEdit,
        db: &mut SqliteConnection,
    ) -> Result<u64> {
        const INIT: &str = "UPDATE book SET ";

        let mut builder = sqlx::QueryBuilder::new(INIT);

        let mut sep = builder.separated(", ");
        if let Some(value) = edit.title.as_ref() {
            sep.push_unseparated("title = ").push_bind(value);
        }

        if let Some(value) = edit.original_title.as_ref() {
            sep.push_unseparated("original_title = ").push_bind(value);
        }

        if let Some(value) = edit.description.as_ref() {
            sep.push_unseparated("description = ").push_bind(value);
        }

        if let Some(value) = edit.rating.as_ref() {
            sep.push_unseparated("rating = ").push_bind(value);
        }

        if let Some(value) = edit.available_at.as_ref() {
            sep.push_unseparated("available_at = ").push_bind(value);
        }

        if let Some(value) = edit.year.as_ref() {
            sep.push_unseparated("year = ").push_bind(value);
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

        if builder.sql() == INIT {
            return Ok(0);
        }

        builder.push(" WHERE id = ").push_bind(book_id);

        Ok(builder.build().execute(db).await?.rows_affected())
    }

    // Search
    fn gen_search_query(filter: &FilterContainer, library: Option<LibraryId>) -> String {
        let mut sql = String::from("SELECT * FROM book WHERE ");
        let orig_len = sql.len();

        let mut f_comp = Vec::new();

        // TODO: Remove Hardcoded value
        f_comp.push(format!(
            "(type_of = {} OR type_of = {}) ",
            BookType::Book as u8,
            BookType::ComicBook as u8,
        ));

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
        offset: i64,
        limit: i64,
        db: &mut SqliteConnection,
    ) -> Result<Vec<Self>> {
        let mut sql = Self::gen_search_query(filter, library);

        sql += "LIMIT $1 OFFSET $2";

        Ok(sqlx::query_as(&sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(db)
            .await?)
    }

    pub async fn count_search_by(
        filter: &FilterContainer,
        library: Option<LibraryId>,
        db: &mut SqliteConnection,
    ) -> Result<i64> {
        let sql = Self::gen_search_query(filter, library).replace("SELECT *", "SELECT COUNT(*)");

        Ok(sqlx::query_scalar(&sql).fetch_one(db).await?)
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
