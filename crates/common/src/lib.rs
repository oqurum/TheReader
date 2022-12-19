use std::path::PathBuf;

use chrono::{DateTime, NaiveDate, Utc};
use common::{Agent, BookId, ImageId, MemberId, PersonId, Source, ThumbnailStore};
use serde::{Deserialize, Serialize};

pub mod error;
mod ext;
mod http;
pub mod specific;
pub mod util;
pub mod reader;

pub use error::{Error, Result};
pub use ext::*;
pub use http::*;
pub use specific::*;

// Member

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Member {
    pub id: MemberId,

    pub name: String,
    pub email: String,

    pub type_of: MemberAuthType,

    pub permissions: Permissions,
    pub preferences: Option<String>,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl Member {
    pub fn parse_preferences(&self) -> Result<Option<MemberPreferences>> {
        let Some(pref) = &self.preferences else {
            return Ok(None);
        };

        Ok(Some(serde_json::from_str(pref)?))
    }
}

// Used for People View

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Person {
    pub id: PersonId,

    pub source: Source,

    pub name: String,
    pub description: Option<String>,
    pub birth_date: Option<NaiveDate>,

    pub thumb_url: ThumbnailStore,

    pub updated_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

impl Person {
    pub fn get_thumb_url(&self) -> String {
        if self.thumb_url != ThumbnailStore::None {
            format!("/api/person/{}/thumbnail", self.id)
        } else {
            String::from("/images/missingperson.jpg")
        }
    }
}

impl PartialEq for Person {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Collection {
    pub id: CollectionId,

    pub member_id: MemberId,

    pub name: String,
    pub description: Option<String>,

    pub thumb_url: ThumbnailStore,

    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

// Used for Library View

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayItem {
    pub id: BookId,

    pub title: String,
    pub cached: BookItemCached,
    pub thumb_path: ThumbnailStore,
}

impl PartialEq for DisplayItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

impl From<DisplayBookItem> for DisplayItem {
    fn from(val: DisplayBookItem) -> Self {
        DisplayItem {
            id: val.id,
            title: val.title.or(val.original_title).unwrap_or_default(),
            cached: val.cached,
            thumb_path: val.thumb_path,
        }
    }
}

// Used for Media View

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct DisplayBookItem {
    pub id: BookId,

    pub library_id: LibraryId,

    pub source: Source,
    pub file_item_count: i64,
    pub title: Option<String>,
    pub original_title: Option<String>,
    pub description: Option<String>,
    pub rating: f64,
    pub thumb_path: ThumbnailStore,

    // TODO: Make table for all tags. Include publisher in it. Remove country.
    pub cached: BookItemCached,

    pub refreshed_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,

    pub available_at: Option<i64>,
    pub year: Option<i64>,
}

impl DisplayBookItem {
    pub fn get_title(&self) -> String {
        self.title
            .as_ref()
            .or(self.original_title.as_ref())
            .cloned()
            .unwrap_or_else(|| String::from("No Title"))
    }
}

impl Default for DisplayBookItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            library_id: Default::default(),
            source: Source {
                agent: Agent::new_owned(String::default()),
                value: String::default(),
            },
            file_item_count: Default::default(),
            title: Default::default(),
            original_title: Default::default(),
            description: Default::default(),
            rating: Default::default(),
            thumb_path: ThumbnailStore::None,
            cached: Default::default(),
            refreshed_at: Utc::now(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
            deleted_at: Default::default(),
            available_at: Default::default(),
            year: Default::default(),
        }
    }
}

// Used for Reader

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaItem {
    pub id: FileId,

    pub path: String,

    pub file_name: String,
    pub file_type: String,
    pub file_size: i64,

    pub library_id: LibraryId,
    pub book_id: Option<BookId>,
    pub chapter_count: usize,

    pub identifier: Option<String>,
    pub hash: String,

    pub modified_at: i64,
    pub accessed_at: i64,
    pub created_at: i64,
    pub deleted_at: Option<i64>,
}

impl PartialEq for MediaItem {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
    }
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq)]
pub enum Progression {
    Ebook {
        chapter: i64,
        char_pos: i64,
        page: i64,
    },

    AudioBook {
        chapter: i64,
        seek_pos: i64,
    },

    Complete,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chapter {
    pub file_path: PathBuf,
    pub value: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryColl {
    pub id: LibraryId,
    pub name: String,

    pub scanned_at: i64,
    pub created_at: i64,
    pub updated_at: i64,

    pub directories: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BasicLibrary {
    pub id: Option<LibraryId>,
    pub name: Option<String>,

    pub directories: Option<Vec<String>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BasicDirectory {
    pub library_id: LibraryId,
    pub path: String,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct BookItemCached {
    pub author: Option<String>,
    pub publisher: Option<String>,
}

impl BookItemCached {
    pub fn as_string(&self) -> String {
        serde_urlencoded::to_string(&self).unwrap()
    }

    /// Returns `None` if string is empty.
    pub fn as_string_optional(&self) -> Option<String> {
        Some(self.as_string()).filter(|v| !v.is_empty())
    }

    pub fn from_string<V: AsRef<str>>(value: V) -> Self {
        serde_urlencoded::from_str(value.as_ref()).unwrap()
    }

    pub fn overwrite_with(&mut self, value: Self) {
        if value.author.is_some() {
            self.author = value.author;
        }

        if value.publisher.is_some() {
            self.publisher = value.publisher;
        }
    }

    pub fn author(mut self, value: String) -> Self {
        self.author = Some(value);
        self
    }

    pub fn publisher(mut self, value: String) -> Self {
        self.publisher = Some(value);
        self
    }

    pub fn author_optional(mut self, value: Option<String>) -> Self {
        if value.is_some() {
            self.author = value;
        }

        self
    }

    pub fn publisher_optional(mut self, value: Option<String>) -> Self {
        if value.is_some() {
            self.publisher = value;
        }

        self
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SearchType {
    Book,
    Person,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SearchFor {
    Book(SearchForBooksBy),
    Person,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SearchForBooksBy {
    Query,
    Title,
    AuthorName,
    Contents,
}

// Image

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Poster {
    pub id: Option<ImageId>,

    pub selected: bool,

    pub path: String,

    pub created_at: DateTime<Utc>,
}
