use std::{collections::HashMap, path::PathBuf};

use chrono::NaiveDate;
use common::{api::QueryListResponse, BookId, Either, ImageId, MemberId, PersonId, Source};
use serde::{Deserialize, Serialize};

use crate::{
    filter::FilterContainer, setup::Config, BasicLibrary, BookEdit, Chapter, Collection,
    DisplayBookItem, DisplayItem, LibraryColl, LibraryId, MediaItem, Member, ModifyValuesBy,
    Person, Poster, Progression, Result, SearchType,
};

// API Routes

// Files
/// GET     /file/{id}
pub type ApiGetFileByIdResponse = Option<self::GetFileByIdResponse>;
/// GET     /file/{id}/res/{tail:.*}
pub type ApiGetFileResourceByIdResponse = String;
/// GET     /file/{id}/pages/{pages}
pub type ApiGetFilePagesByIdResponse = self::GetChaptersResponse;
/// GET     /file/{id}/debug/{tail:.*}
pub type ApiGetFileDebugByIdResponse = String;
/// POST    /file/{id}/progress
pub type ApiPostFileProgressByIdResponse = ();
/// DELETE  /file/{id}/progress
pub type ApiDeleteFileProgressByIdResponse = ();
/// GET     /file/{id}/notes
pub type ApiGetFileNotesByIdResponse = Option<String>;
/// POST    /file/{id}/notes
pub type ApiPostFileNotesByIdResponse = ();
/// DELETE  /file/{id}/notes
pub type ApiDeleteFileNotesByIdResponse = ();

// IMAGES
/// GET     /image/{type}/{id}
pub type ApiGetImageTypeByIdResponse = Vec<u8>;

// Libraries
/// GET     /libraries
pub type ApiGetLibrariesResponse = self::GetLibrariesResponse;
/// GET     /library/{id}
pub type ApiGetLibraryIdResponse = LibraryColl;

// Collections
/// GET     /collections
pub type ApiGetCollectionListResponse = Vec<Collection>;
/// GET     /collection/{id}
pub type ApiGetCollectionIdResponse = Collection;
/// GET     /collection/{id}/books
pub type ApiGetCollectionIdBooksResponse = self::GetBookListResponse;

// Members
/// GET     /member
pub type ApiGetMemberSelfResponse = self::GetMemberSelfResponse;
/// GET     /members
pub type ApiGetMembersListResponse = self::GetMembersListResponse;

// Books
/// GET     /books
pub type ApiGetBookListResponse = self::GetBookListResponse;
/// GET     /books/preset
pub type ApiGetBookPresetListResponse = self::GetBookPresetListResponse;
/// GET     /book/{id}/posters
pub type ApiGetPosterByBookIdResponse = self::GetPostersResponse;
/// POST    /book/{id}/posters
pub type ApiPostPosterByBookIdResponse = String;
/// GET     /book/{id}
pub type ApiGetBookByIdResponse = self::GetBookResponse;
/// POST    /book/{id}
pub type ApiPostUpdateBookResponse = ();
/// GET     /book/search
pub type ApiGetBookSearchResponse = self::BookSearchResponse;
/// GET     /book/progress
pub type ApiGetBookProgressResponse = Option<Progression>;

// Directory
/// GET     /book/search
pub type ApiGetDirectoryResponse = GetDirectoryResponse;

// Options
/// GET     /options
pub type ApiGetOptionsResponse = self::GetOptionsResponse;
/// POST    /options
pub type ApiPostOptionsAddResponse = ();
/// DELETE  /options
pub type ApiPostOptionsRemoveResponse = ();

// People
/// GET     /people
pub type ApiGetPeopleResponse = self::GetPeopleResponse;
// TODO: Remove? Use /image/{type}/{id}?
/// GET     /person/{id}/thumbnail
pub type ApiGetPersonThumbnailResponse = Vec<u8>;
/// POST    /person/{id}
pub type ApiPostUpdatePersonResponse = ();

// Task
/// POST    /task
pub type ApiPostRunTaskResponse = ();

// Setup
/// GET     /setup
pub type ApiGetSetupResponse = Config;
/// POST    /setup
pub type ApiPostSetupResponse = ();

// Directory

#[derive(Serialize, Deserialize)]
pub struct DirectoryEntry {
    pub title: String,
    pub path: PathBuf,
    pub is_file: bool,
}

#[derive(Serialize, Deserialize)]
pub struct GetDirectoryResponse {
    pub path: PathBuf,
    pub items: Vec<DirectoryEntry>,
}

#[derive(Serialize, Deserialize)]
pub struct GetDirectoryQuery {
    pub path: String,
}

// Images

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetPostersResponse {
    pub items: Vec<Poster>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ChangePosterBody {
    pub url_or_id: Either<String, ImageId>,
}

// Members

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetMemberSelfResponse {
    pub member: Option<Member>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetMembersListResponse {
    pub count: usize,
    pub items: Vec<Member>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum UpdateMember {
    Delete { id: MemberId },

    Invite { email: String },
}

// Collections

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct NewCollectionBody {
    pub name: String,
    pub description: Option<String>,
}

// Libraries

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetLibrariesResponse {
    pub items: Vec<LibraryColl>,
}

#[derive(Debug, Default, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct UpdateLibrary {
    pub name: Option<String>,
    pub is_public: Option<bool>,

    pub add_directories: Vec<String>,
    pub remove_directories: Vec<String>,
}

// Book
#[derive(Default, Debug, PartialEq, Eq, Serialize, Deserialize, Clone)]
pub struct MassEditBooks {
    pub book_ids: Vec<BookId>,

    // People
    pub people_list: Vec<PersonId>,
    pub people_list_mod: ModifyValuesBy,
}

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct LoadResourceQuery {
    #[serde(default)]
    pub configure_pages: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetFileByIdResponse {
    pub media: MediaItem,
    pub progress: Option<Progression>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetBookListResponse {
    pub count: usize,
    pub items: Vec<DisplayItem>,
}

#[derive(Serialize, Deserialize)]
pub struct BookListQuery {
    pub library: Option<LibraryId>,

    pub offset: Option<i64>,
    pub limit: Option<i64>,

    pub filters: Option<FilterContainer>,
}

impl BookListQuery {
    pub fn new(
        library: Option<LibraryId>,
        offset: Option<i64>,
        limit: Option<i64>,
        filters: Option<FilterContainer>,
    ) -> Result<Self> {
        Ok(Self {
            library,
            offset,
            limit,
            filters,
        })
    }

    pub fn has_query(&self) -> bool {
        self.filters
            .as_ref()
            .map(|v| !v.filters.is_empty())
            .unwrap_or_default()
    }
}

#[derive(Serialize, Deserialize)]
pub struct BookPresetListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,

    pub preset: BookPresetListType,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub enum BookPresetListType {
    Progressing,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetBookPresetListResponse {
    pub count: usize,
    pub items: Vec<BookProgression>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookProgression {
    pub progress: Progression,
    pub book: DisplayItem,
    pub file: MediaItem,
}

pub type GetChaptersResponse = QueryListResponse<Chapter>;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileUnwrappedInfo {
    pub header_items: Vec<FileUnwrappedHeaderType>,
    /// The combined Hash of <style text()>, <link @href>,
    pub header_hash: String,
    pub inner_body: String,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct FileUnwrappedHeaderType {
    pub name: String,
    pub attributes: Vec<(String, String)>,
    pub chars: Option<String>,
}

// People

pub type GetPeopleResponse = QueryListResponse<Person>;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PostPersonBody {
    AutoMatchById,

    UpdateBySource(Source),

    CombinePersonWith(PersonId),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetPeopleSearch {
    pub query: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetPersonResponse {
    pub person: Person,
}

// Options

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetOptionsResponse {
    pub libraries: Vec<LibraryColl>,
    pub config: Option<Config>,
}

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct ModifyOptionsBody {
    pub library: Option<BasicLibrary>,

    pub libby_public_search: Option<bool>,
}

// Metadata

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct GetBookResponse {
    pub book: DisplayBookItem,
    pub media: Vec<MediaItem>,
    pub progress: Vec<Option<Progression>>,
    pub people: Vec<Person>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PostBookBody {
    UnMatch,

    RefreshBookId,
    AutoMatchBookId,
    AutoMatchBookIdByFiles,

    UpdateBookBySource(Source),

    Edit(BookEdit),
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetBookSearch {
    pub query: String,
    pub search_type: SearchType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BookSearchResponse {
    pub items: HashMap<String, Vec<SearchItem>>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum SearchItem {
    Book(MetadataBookSearchItem),
    Person(MetadataPersonSearchItem),
}

impl SearchItem {
    pub fn as_book(&self) -> &MetadataBookSearchItem {
        match self {
            Self::Book(v) => v,
            _ => unreachable!(),
        }
    }

    pub fn as_person(&self) -> &MetadataPersonSearchItem {
        match self {
            Self::Person(v) => v,
            _ => unreachable!(),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetadataPersonSearchItem {
    pub source: Source,

    pub cover_image: Option<String>,

    pub name: String,
    pub other_names: Option<Vec<String>>,
    pub description: Option<String>,

    pub birth_date: Option<NaiveDate>,
    pub death_date: Option<NaiveDate>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetadataBookSearchItem {
    pub source: Source,
    pub author: Option<String>,
    pub thumbnail_url: String,
    pub description: Option<String>,
    pub name: String,
}

// Task

#[derive(Debug, Default, Serialize, Deserialize, Clone)]
pub struct RunTaskBody {
    pub run_search: Option<LibraryId>,
    pub run_metadata: Option<LibraryId>,
}

#[derive(Deserialize)]
pub struct SimpleListQuery {
    pub offset: Option<i64>,
    pub limit: Option<i64>,
    pub query: Option<String>,
}
