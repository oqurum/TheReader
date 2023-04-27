use std::{
    collections::HashMap,
    ops::{Deref, DerefMut},
};

use crate::{
    config::get_config,
    model::{
        BookPersonModel, FileModel, NewBookModel, NewPersonModel, PersonAltModel, PersonModel,
    },
    util, Result,
};
use async_trait::async_trait;
use chrono::{NaiveDate, TimeZone, Utc};
use common::{Agent, Either, PersonId, Source, ThumbnailStore};
use common_local::{BookItemCached, LibraryId, SearchFor};
use futures::Future;
use sqlx::SqliteConnection;
use tracing::error;

use self::{
    google_books::GoogleBooksMetadata, libby::LibbyMetadata, local::LocalMetadata,
    openlibrary::OpenLibraryMetadata,
};

pub mod google_books;
pub mod libby;
pub mod local;
pub mod openlibrary;

// "source" column: [prefix]:[id]

/// Simple return if found, If error then log it.
macro_rules! return_if_found {
    ($v: expr) => {
        match $v {
            Ok(Some(v)) => return Ok(Some(v)),
            Ok(None) => (),
            Err(error) => error!(?error),
        }
    };
}

macro_rules! return_if_found_vec {
    ($v: expr) => {
        match $v {
            Ok(v) if v.is_empty() => (),
            Ok(v) => return Ok(v),
            Err(error) => error!(?error),
        }
    };
}

pub struct ActiveAgents {
    pub google: bool,
    pub libby: bool,
    pub local: bool,
    pub openlib: bool,
}

impl Default for ActiveAgents {
    fn default() -> Self {
        Self {
            google: true,
            libby: true,
            local: true,
            openlib: true,
        }
    }
}

#[async_trait]
pub trait Metadata {
    fn prefix_text<V: AsRef<str>>(&self, value: V) -> String {
        format!("{}:{}", self.get_agent(), value.as_ref())
    }

    fn get_agent(&self) -> Agent;

    // Metadata
    async fn get_metadata_from_files(
        &mut self,
        files: &[FileModel],
    ) -> Result<Option<MetadataReturned>>;

    async fn get_metadata_by_source_id(
        &mut self,
        _value: &str,
    ) -> Result<Option<MetadataReturned>> {
        Ok(None)
    }

    // Person

    async fn get_person_by_source_id(&mut self, _value: &str) -> Result<Option<AuthorInfo>> {
        Ok(None)
    }

    // Both

    async fn search(&mut self, _search: &str, _search_for: SearchFor) -> Result<Vec<SearchItem>> {
        Ok(Vec::new())
    }
}

// TODO: Utilize current metadata in get_metadata_from_files.
// TODO: Order which metadata should be tried.
/// Attempts to return the first valid Metadata from Files.
///
/// Also checks local agent.
pub async fn get_metadata_from_files(
    files: &[FileModel],
    agent: &ActiveAgents,
) -> Result<Option<MetadataReturned>> {
    let config = get_config();

    if agent.libby && config.authenticators.main_server && config.libby.token.is_some() {
        return_if_found!(LibbyMetadata.get_metadata_from_files(files).await);
    }

    if agent.google {
        return_if_found!(GoogleBooksMetadata.get_metadata_from_files(files).await);
    }

    if agent.openlib {
        return_if_found!(OpenLibraryMetadata.get_metadata_from_files(files).await);
    }

    if agent.local {
        // TODO: Don't re-scan file if we already have metadata from file.
        return_if_found!(LocalMetadata.get_metadata_from_files(files).await);
    }

    Ok(None)
}

/// Doesn't check local
pub async fn get_metadata_by_source(source: &Source) -> Result<Option<MetadataReturned>> {
    let config = get_config();

    match &source.agent {
        v if v == &LibbyMetadata.get_agent()
            && config.authenticators.main_server
            && config.libby.token.is_some() =>
        {
            LibbyMetadata.get_metadata_by_source_id(&source.value).await
        }
        v if v == &OpenLibraryMetadata.get_agent() => {
            OpenLibraryMetadata
                .get_metadata_by_source_id(&source.value)
                .await
        }
        v if v == &GoogleBooksMetadata.get_agent() => {
            GoogleBooksMetadata
                .get_metadata_by_source_id(&source.value)
                .await
        }

        _ => Ok(None),
    }
}

/// Attempts to return the first valid Metadata from Query.
pub async fn search_and_return_first_valid_agent(
    query: &str,
    search_for: SearchFor,
    agent: &ActiveAgents,
) -> Result<Vec<SearchItem>> {
    let config = get_config();

    if agent.libby && config.authenticators.main_server && config.libby.token.is_some() {
        return_if_found_vec!(LibbyMetadata.search(query, search_for).await);
    }

    if agent.google {
        return_if_found_vec!(GoogleBooksMetadata.search(query, search_for).await);
    }

    if agent.openlib {
        return_if_found_vec!(OpenLibraryMetadata.search(query, search_for).await);
    }

    Ok(Vec::new())
}

/// Searches all agents except for local.
pub async fn search_all_agents(
    search: &str,
    search_for: SearchFor,
    agent: &ActiveAgents,
) -> Result<SearchResults> {
    let config = get_config();

    let mut map = HashMap::new();

    // Checks to see if we can use get_metadata_by_source (source:id)
    if let Ok(source) = Source::try_from(search) {
        // Check if it's a Metadata Source.
        if let Some(val) = get_metadata_by_source(&source).await? {
            map.insert(source.agent, vec![SearchItem::Book(val.meta)]);

            return Ok(SearchResults(map));
        }
    }

    async fn search_or_ignore(
        enabled: bool,
        value: impl Future<Output = Result<Vec<SearchItem>>>,
    ) -> Result<Vec<SearchItem>> {
        if enabled {
            value.await
        } else {
            Ok(Vec::new())
        }
    }

    // Search all sources
    let prefixes = [
        LibbyMetadata.get_agent(),
        OpenLibraryMetadata.get_agent(),
        GoogleBooksMetadata.get_agent(),
    ];
    let asdf = futures::future::join_all([
        search_or_ignore(
            agent.libby && config.authenticators.main_server && config.libby.token.is_some(),
            LibbyMetadata.search(search, search_for),
        ),
        search_or_ignore(
            agent.openlib,
            OpenLibraryMetadata.search(search, search_for),
        ),
        search_or_ignore(agent.google, GoogleBooksMetadata.search(search, search_for)),
    ])
    .await;

    for (val, prefix) in asdf.into_iter().zip(prefixes) {
        match val {
            Ok(val) => {
                if !val.is_empty() {
                    map.insert(prefix, val);
                }
            }

            Err(error) => error!(?error),
        }
    }

    Ok(SearchResults(map))
}

/// Searches all agents except for local.
pub async fn get_person_by_source(source: &Source) -> Result<Option<AuthorInfo>> {
    let config = get_config();

    match &source.agent {
        v if v == &LibbyMetadata.get_agent()
            && config.authenticators.main_server
            && config.libby.token.is_some() =>
        {
            LibbyMetadata.get_person_by_source_id(&source.value).await
        }
        v if v == &OpenLibraryMetadata.get_agent() => {
            OpenLibraryMetadata
                .get_person_by_source_id(&source.value)
                .await
        }
        v if v == &GoogleBooksMetadata.get_agent() => {
            GoogleBooksMetadata
                .get_person_by_source_id(&source.value)
                .await
        }

        _ => Ok(None),
    }
}

pub struct SearchResults(pub HashMap<Agent, Vec<SearchItem>>);

impl SearchResults {
    pub fn sort_items_by_similarity(self, match_with: &str) -> Vec<(f64, SearchItem)> {
        util::sort_by_similarity(match_with, self.0.into_values().flatten(), |v| match v {
            SearchItem::Book(v) => v.title.as_deref(),
            SearchItem::Author(v) => Some(&v.name),
        })
    }
}

impl Deref for SearchResults {
    type Target = HashMap<Agent, Vec<SearchItem>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for SearchResults {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

#[derive(Debug)]
pub enum SearchItem {
    Author(AuthorInfo),
    Book(FoundItem),
}

impl SearchItem {
    pub fn into_author(self) -> Option<AuthorInfo> {
        match self {
            SearchItem::Author(v) => Some(v),
            _ => None,
        }
    }

    pub fn into_book(self) -> Option<FoundItem> {
        match self {
            SearchItem::Book(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_book(&self) -> Option<&FoundItem> {
        match self {
            SearchItem::Book(v) => Some(v),
            _ => None,
        }
    }
}

#[derive(Debug)]
pub struct AuthorInfo {
    pub source: Source,

    pub cover_image_url: Option<FoundImageLocation>,

    pub name: String,
    pub other_names: Option<Vec<String>>,
    pub description: Option<String>,

    pub birth_date: Option<NaiveDate>,
    pub death_date: Option<NaiveDate>,
}

#[derive(Debug)]
pub struct MetadataReturned {
    // Person, Alt Names
    pub authors: Option<Vec<AuthorInfo>>,
    pub publisher: Option<String>, // TODO: Is this needed? We have BookItemCached in meta field
    // TODO: Add More.
    pub meta: FoundItem,
}

impl MetadataReturned {
    /// Returns (Main Author, Person IDs)
    pub async fn add_or_ignore_authors_into_database(
        &mut self,
        db: &mut SqliteConnection,
    ) -> Result<(Option<String>, Vec<PersonId>)> {
        let mut main_author = None;
        let mut person_ids = Vec::new();

        if let Some(authors_with_alts) = self.authors.take() {
            for author_info in authors_with_alts {
                let mut relink_books = Vec::new();

                // Check if we already have a person by that name anywhere in the two database tables.
                if let Some(person) = PersonModel::find_one_by_name(&author_info.name, db).await? {
                    // Check if it's from the same source.
                    // If not, we remove the old one and replace it with the new one.
                    if author_info.source != person.source {
                        PersonAltModel::delete_by_id(person.id, db).await?;
                        PersonModel::delete_by_id(person.id, db).await?;

                        relink_books =
                            BookPersonModel::find_by(Either::Right(person.id), db).await?;
                        BookPersonModel::delete_by_person_id(person.id, db).await?;
                    } else {
                        person_ids.push(person.id);

                        if main_author.is_none() {
                            main_author = Some(person.name);
                        }

                        continue;
                    }
                }

                let mut thumb_url = ThumbnailStore::None;

                // Download thumb url and store it.
                if let Some(mut url) = author_info.cover_image_url {
                    url.download(db).await?;

                    if let FoundImageLocation::Local(path) = url {
                        thumb_url = path;
                    }
                }

                let author = NewPersonModel {
                    source: author_info.source,
                    name: author_info.name,
                    description: author_info.description,
                    birth_date: author_info.birth_date,
                    thumb_url,
                    // TODO: death_date: author_info.death_date,
                    updated_at: Utc::now().naive_utc(),
                    created_at: Utc::now().naive_utc(),
                };

                let person = author.insert(db).await?;

                if let Some(alts) = author_info.other_names {
                    for name in alts {
                        // Ignore errors. Errors should just be UNIQUE constraint failed
                        if let Err(e) = (PersonAltModel {
                            person_id: person.id,
                            name,
                        })
                        .insert(db)
                        .await
                        {
                            error!("[OL]: Add Alt Name Error: {e}");
                        }
                    }
                }

                for model in relink_books {
                    BookPersonModel {
                        person_id: person.id,
                        book_id: model.book_id,
                    }
                    .insert_or_ignore(db)
                    .await?;
                }

                person_ids.push(person.id);

                if main_author.is_none() {
                    main_author = Some(person.name);
                }
            }
        }

        Ok((main_author, person_ids))
    }
}

#[derive(Debug)]
pub struct FoundItem {
    pub source: Source,
    pub title: Option<String>,
    pub description: Option<String>,
    pub rating: f64,

    pub thumb_locations: Vec<FoundImageLocation>,

    // TODO: Make table for all tags. Include publisher in it. Remove country.
    pub cached: BookItemCached,

    pub available_at: Option<i64>,
    pub year: Option<i64>,
}

impl From<FoundItem> for NewBookModel {
    fn from(val: FoundItem) -> Self {
        NewBookModel {
            library_id: LibraryId::none(),
            type_of: 1.try_into().unwrap(),
            source: val.source,
            file_item_count: 1,
            title: val.title.clone(),
            original_title: val.title,
            description: val.description,
            rating: val.rating,
            thumb_url: val
                .thumb_locations
                .iter()
                .find_map(|v| v.as_local_value().cloned())
                .unwrap_or(ThumbnailStore::None),
            // all_thumb_urls: val
            //     .thumb_locations
            //     .into_iter()
            //     .filter_map(|v| v.into_url_value())
            //     .collect(),
            cached: val.cached,
            refreshed_at: Utc::now().naive_utc(),
            created_at: Utc::now().naive_utc(),
            updated_at: Utc::now().naive_utc(),
            deleted_at: None,
            available_at: val
                .available_at
                .map(|v| Utc.timestamp_millis_opt(v).unwrap().naive_utc()),
            year: val.year,
            parent_id: None,
            index: None,
        }
    }
}

#[derive(Debug)]
pub enum FoundImageLocation {
    Url(String),
    FileData(Vec<u8>),
    Local(ThumbnailStore),
}

impl FoundImageLocation {
    pub fn into_url_value(self) -> Option<String> {
        match self {
            Self::Url(v) => Some(v),
            _ => None,
        }
    }

    pub fn as_url_value(&self) -> Option<&str> {
        match self {
            Self::Url(v) => Some(v.as_str()),
            _ => None,
        }
    }

    pub fn as_local_value(&self) -> Option<&ThumbnailStore> {
        match self {
            Self::Local(v) => Some(v),
            _ => None,
        }
    }

    pub fn is_local(&self) -> bool {
        matches!(self, Self::Local(_))
    }

    pub fn is_file_data(&self) -> bool {
        matches!(self, Self::FileData(_))
    }

    pub fn is_url(&self) -> bool {
        matches!(self, Self::Url(_))
    }

    pub async fn download(&mut self, db: &mut SqliteConnection) -> Result<()> {
        match self {
            FoundImageLocation::Url(url) => {
                // Fix URL
                if url.starts_with("//") {
                    url.insert_str(0, "https:");
                }

                let resp = reqwest::get(&*url).await?.bytes().await?;

                match crate::store_image(resp.to_vec(), db).await {
                    Ok(model) => *self = Self::Local(model.path),
                    Err(e) => error!("FoundImageLocation::download: {}", e),
                }
            }

            FoundImageLocation::FileData(image) => {
                if let Ok(model) = crate::store_image(image.clone(), db).await {
                    *self = Self::Local(model.path)
                }
            }

            _ => (),
        }

        Ok(())
    }
}
