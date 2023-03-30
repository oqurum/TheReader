use std::path::PathBuf;

use chrono::{DateTime, NaiveDate, Utc};
use common::{Agent, BookId, ImageId, MemberId, PersonId, Source, ThumbnailStore};
use http::api::FileUnwrappedInfo;
use num_enum::{TryFromPrimitive, IntoPrimitive};
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

    pub library_access: Option<String>,

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

    pub fn parse_library_access_or_default(&self) -> Result<LibraryAccess> {
        let Some(access) = &self.library_access else {
            return Ok(LibraryAccess::default());
        };

        Ok(serde_json::from_str(access)?)
    }
}

#[derive(Debug, Default, Clone, PartialEq, Serialize, Deserialize)]
pub struct MemberUpdate {
    pub name: Option<String>,
    pub email: Option<String>,
    pub preferences: Option<MemberPreferences>,

    // Only editable by admins
    pub type_of: Option<MemberAuthType>,
    pub permissions: Option<Permissions>,
    pub library_access: Option<LibraryAccess>,
}

impl MemberUpdate {
    pub fn fill_with_member(member: &Member) -> Self {
        Self {
            name: Some(member.name.clone()),
            email: Some(member.email.clone()),
            preferences: member.parse_preferences().unwrap_or_default(),
            type_of: Some(member.type_of),
            permissions: Some(member.permissions),
            library_access: member.parse_library_access_or_default().ok(),
        }
    }
}


#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(untagged)]
pub enum LibraryAccess {
    /// No libraries are accessible.
    None,
    /// All libraries are accessible. Even private ones.
    All,
    /// Only public libraries are accessible.
    #[default]
    AllPublic,
    /// Only specific libraries are accessible.
    Specific(Vec<LibraryId>),
}

impl LibraryAccess {
    pub fn is_accessible(&self, lib_id: LibraryId, is_public: bool) -> bool {
        match self {
            Self::None => false,
            Self::All => true,
            Self::AllPublic => is_public,
            Self::Specific(items) => items.contains(&lib_id),
        }
    }

    pub fn set_viewable(&mut self, lib_id: LibraryId, value: bool, all_libraries: &[LibraryColl]) {
        let Some(library) = all_libraries.iter().find(|v| v.id == lib_id) else {
            return;
        };

        match self {
            Self::None => if value {
                *self = Self::Specific(vec![library.id]);
            },

            Self::All => if !value {
                *self = Self::Specific(all_libraries.iter().filter(|v| v.id != library.id).map(|v| v.id).collect());
            },

            Self::AllPublic => {
                if value {
                    if !library.is_public {
                        let mut viewing = vec![library.id];

                        for library in all_libraries.iter().filter(|v| v.is_public) {
                            viewing.push(library.id);
                        }

                        *self = Self::Specific(viewing);
                    }
                } else if library.is_public {
                    *self = Self::Specific(all_libraries.iter().filter(|v| v.id != library.id && v.is_public).map(|v| v.id).collect());
                }
            },

            Self::Specific(items) => {
                if value {
                    if !items.contains(&library.id) {
                        items.push(library.id);
                    }

                    // If all libraries are accessible, then we can just switch to All
                    if all_libraries.iter().all(|v| items.contains(&v.id)) {
                        *self = Self::All;
                        return;
                    }

                    // If all libraries are public, then we can just switch to AllPublic
                    if all_libraries.iter().filter(|v| v.is_public).all(|v| items.contains(&v.id)) {
                        *self = Self::AllPublic;
                        return;
                    }
                } else if let Some(index) = items.iter().position(|v| v == &library.id) {
                    items.remove(index);
                }

                if items.is_empty() {
                    *self = Self::AllPublic;
                }
            }
        }
    }

    pub fn get_accessible_libraries<'a>(&self, libraries: &'a [LibraryColl]) -> Vec<&'a LibraryColl> {
        let mut items = Vec::new();

        match self {
            Self::None => (),
            Self::All => return libraries.iter().collect(),
            Self::AllPublic => return libraries.iter().filter(|v| v.is_public).collect(),

            Self::Specific(ids) => {
                for id in ids {
                    if let Some(library) = libraries.iter().find(|v| v.id == *id) {
                        items.push(library);
                    }
                }
            }
        }

        items
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
    pub type_of: BookType,

    pub public_source_url: Option<String>,
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

// TODO: Remove.
impl Default for DisplayBookItem {
    fn default() -> Self {
        Self {
            id: Default::default(),
            library_id: Default::default(),
            type_of: BookType::Book,
            public_source_url: None,
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

impl MediaItem {
    pub fn is_comic_book(&self) -> bool {
        LibraryType::ComicBook.is_filetype_valid(&self.file_type)
    }
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

    pub info: FileUnwrappedInfo,
}

#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(i32)]
pub enum LibraryType {
    Book = 1,
    ComicBook = 2,
    // TODO: PDF's. What do they fall under? Documents?
}

impl LibraryType {
    pub fn is_filetype_valid(self, value: &str) -> bool {
        match self {
            LibraryType::Book => ["epub", "mobi", "azw", "azw3", "kfx"].contains(&value),
            LibraryType::ComicBook => ["cbz", "cbr", "cbt", "cba", "cb7"].contains(&value),
        }
    }
}


#[derive(Debug, Serialize, Deserialize, Clone, Copy, PartialEq, Eq, TryFromPrimitive, IntoPrimitive)]
#[repr(i32)]
pub enum BookType {
    Book = 1,

    /// The main book.
    ComicBook = 2,
    /// For either Prologue or Chapter
    ComicBookSection = 3,
    /// The chapters of the book.
    ComicBookChapter = 4,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryColl {
    pub id: LibraryId,
    pub name: String,

    pub type_of: LibraryType,

    pub is_public: bool,
    pub settings: Option<String>,

    pub scanned_at: i64,
    pub created_at: i64,
    pub updated_at: i64,

    pub directories: Vec<String>,
}

// TODO: Rename / remove
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BasicLibrary {
    pub id: Option<LibraryId>,
    pub name: Option<String>,

    pub type_of: LibraryType,

    pub is_public: bool,
    pub settings: Option<String>,

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
        serde_urlencoded::to_string(self).unwrap()
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
