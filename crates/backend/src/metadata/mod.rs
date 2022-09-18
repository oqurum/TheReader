use std::{collections::HashMap, ops::{Deref, DerefMut}};

use crate::{Result, model::{book::BookModel, file::FileModel, person::{PersonModel, NewPersonModel}, person_alt::PersonAltModel}, util};
use async_trait::async_trait;
use common_local::{SearchFor, BookItemCached, LibraryId};
use chrono::Utc;
use common::{BookId, PersonId, ThumbnailStore, Source, Agent};

use crate::database::Database;

use self::{
    google_books::GoogleBooksMetadata,
    libby::LibbyMetadata,
    local::LocalMetadata,
    openlibrary::OpenLibraryMetadata,
};

pub mod google_books;
pub mod libby;
pub mod local;
pub mod openlibrary;

// "source" column: [prefix]:[id]

/// Simple return if found, println if error.
macro_rules! return_if_found {
    ($v: expr) => {
        match $v {
            Ok(Some(v)) => return Ok(Some(v)),
            Ok(None) => (),
            Err(e) => eprintln!("metadata::get_metadata: {}", e)
        }
    };
}

macro_rules! return_if_found_vec {
    ($v: expr) => {
        match $v {
            Ok(v) if v.is_empty() => (),
            Ok(v) => return Ok(v),
            Err(e) => eprintln!("metadata::get_metadata: {}", e)
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
    async fn get_metadata_from_files(&mut self, files: &[FileModel]) -> Result<Option<MetadataReturned>>;

    async fn get_metadata_by_source_id(&mut self, _value: &str) -> Result<Option<MetadataReturned>> {
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
pub async fn get_metadata_from_files(files: &[FileModel], agent: &ActiveAgents) -> Result<Option<MetadataReturned>> {
    if agent.libby {
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
    match &source.agent {
        v if v == &LibbyMetadata.get_agent() => LibbyMetadata.get_metadata_by_source_id(&source.value).await,
        v if v == &OpenLibraryMetadata.get_agent() => OpenLibraryMetadata.get_metadata_by_source_id(&source.value).await,
        v if v == &GoogleBooksMetadata.get_agent() => GoogleBooksMetadata.get_metadata_by_source_id(&source.value).await,

        _ => Ok(None)
    }
}



/// Attempts to return the first valid Metadata from Query.
pub async fn search_and_return_first_valid_agent(query: &str, search_for: SearchFor, agent: &ActiveAgents) -> Result<Vec<SearchItem>> {
    if agent.libby {
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
pub async fn search_all_agents(search: &str, search_for: SearchFor) -> Result<SearchResults> {
    let mut map = HashMap::new();

    // Checks to see if we can use get_metadata_by_source (source:id)
    if let Ok(source) = Source::try_from(search) {
        // Check if it's a Metadata Source.
        if let Some(val) = get_metadata_by_source(&source).await? {
            map.insert(
                source.agent,
                vec![SearchItem::Book(val.meta)],
            );

            return Ok(SearchResults(map));
        }
    }

    // Search all sources
    let prefixes = [LibbyMetadata.get_agent(), OpenLibraryMetadata.get_agent(), GoogleBooksMetadata.get_agent()];
    let asdf = futures::future::join_all(
        [LibbyMetadata.search(search, search_for), OpenLibraryMetadata.search(search, search_for), GoogleBooksMetadata.search(search, search_for)]
    ).await;

    for (val, prefix) in asdf.into_iter().zip(prefixes) {
        match val {
            Ok(val) => {
                map.insert(
                    prefix,
                    val,
                );
            }

            Err(e) => eprintln!("{:?}", e),
        }
    }

    Ok(SearchResults(map))
}

/// Searches all agents except for local.
pub async fn get_person_by_source(source: &Source) -> Result<Option<AuthorInfo>> {
    match &source.agent {
        v if v == &LibbyMetadata.get_agent() => LibbyMetadata.get_person_by_source_id(&source.value).await,
        v if v == &OpenLibraryMetadata.get_agent() => OpenLibraryMetadata.get_person_by_source_id(&source.value).await,
        v if v == &GoogleBooksMetadata.get_agent() => GoogleBooksMetadata.get_person_by_source_id(&source.value).await,

        _ => Ok(None)
    }
}



pub struct SearchResults(pub HashMap<Agent, Vec<SearchItem>>);

impl SearchResults {
    pub fn sort_items_by_similarity(self, match_with: &str) -> Vec<(f64, SearchItem)> {
        util::sort_by_similarity(
            match_with,
            self.0.into_values().flatten(),
            |v| {
                match v {
                    SearchItem::Book(v) => v.title.as_deref(),
                    SearchItem::Author(v) => Some(&v.name),
                }
            }
        )
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
    Book(FoundItem)
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
}


#[derive(Debug)]
pub struct AuthorInfo {
    pub source: Source,

    pub cover_image_url: Option<String>,

    pub name: String,
    pub other_names: Option<Vec<String>>,
    pub description: Option<String>,

    pub birth_date: Option<String>,
    pub death_date: Option<String>,
}


#[derive(Debug)]
pub struct MetadataReturned {
    // Person, Alt Names
    pub authors: Option<Vec<AuthorInfo>>,
    pub publisher: Option<String>,
    // TODO: Add More.

    pub meta: FoundItem
}

impl MetadataReturned {
    /// Returns (Main Author, Person IDs)
    pub async fn add_or_ignore_authors_into_database(&mut self, db: &Database) -> Result<(Option<String>, Vec<PersonId>)> {
        let mut main_author = None;
        let mut person_ids = Vec::new();

        if let Some(authors_with_alts) = self.authors.take() {
            for author_info in authors_with_alts {
                // Check if we already have a person by that name anywhere in the two database tables.
                if let Some(person) = PersonModel::find_one_by_name(&author_info.name, db).await? {
                    person_ids.push(person.id);

                    if main_author.is_none() {
                        main_author = Some(person.name);
                    }

                    continue;
                }

                let mut thumb_url = ThumbnailStore::None;

                // Download thumb url and store it.
                if let Some(url) = author_info.cover_image_url {
                    let resp = reqwest::get(url).await?;

                    if resp.status().is_success() {
                        let bytes = resp.bytes().await?;

                        match crate::store_image(bytes.to_vec(), db).await {
                            Ok(model) => thumb_url = model.path,
                            Err(e) => {
                                eprintln!("add_or_ignore_authors_into_database Error: {}", e);
                            }
                        }
                    } else {
                        let text = resp.text().await;
                        eprintln!("add_or_ignore_authors_into_database Error: {:?}", text);
                    }
                }

                let author = NewPersonModel {
                    source: author_info.source,
                    name: author_info.name,
                    description: author_info.description,
                    birth_date: author_info.birth_date,
                    thumb_url,
                    // TODO: death_date: author_info.death_date,
                    updated_at: Utc::now(),
                    created_at: Utc::now(),
                };

                let person = author.insert(db).await?;

                if let Some(alts) = author_info.other_names {
                    for name in alts {
                        // Ignore errors. Errors should just be UNIQUE constraint failed
                        if let Err(e) = (PersonAltModel {
                            person_id: person.id,
                            name,
                        }).insert(db).await {
                            eprintln!("[OL]: Add Alt Name Error: {e}");
                        }
                    }
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
    pub year: Option<i64>
}

impl From<FoundItem> for BookModel {
    fn from(val: FoundItem) -> Self {
        BookModel {
            id: BookId::none(),
            library_id: LibraryId::none(),
            source: val.source,
            file_item_count: 1,
            title: val.title.clone(),
            original_title: val.title,
            description: val.description,
            rating: val.rating,
            thumb_path: val.thumb_locations.iter()
                .find_map(|v| v.as_local_value().cloned())
                .unwrap_or(ThumbnailStore::None),
            all_thumb_urls: val.thumb_locations.into_iter().filter_map(|v| v.into_url_value()).collect(),
            cached: val.cached,
            refreshed_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: None,
            available_at: val.available_at,
            year: val.year,
            hash: String::new(),
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
            _ => None
        }
    }

    pub fn as_url_value(&self) -> Option<&str> {
        match self {
            Self::Url(v) => Some(v.as_str()),
            _ => None
        }
    }

    pub fn as_local_value(&self) -> Option<&ThumbnailStore> {
        match self {
            Self::Local(v) => Some(v),
            _ => None
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

    pub async fn download(&mut self, db: &Database) -> Result<()> {
        match self {
            FoundImageLocation::Url(ref url) => {
                let resp = reqwest::get(url)
                    .await?
                    .bytes()
                    .await?;

                let model = crate::store_image(resp.to_vec(), db).await?;

                *self = Self::Local(model.path);
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