use std::collections::HashMap;

use common::{ImageId, PersonId, Either, Source, api::QueryListResponse, BookId};
use serde::{Serialize, Deserialize};

use crate::{MediaItem, Progression, LibraryColl, BasicLibrary, Chapter, DisplayItem, DisplayBookItem, Person, SearchType, Member, Poster, Result, LibraryId, filter::FilterContainer, BookEdit, ModifyValuesBy, setup::Config};


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

// Members
/// GET     /member
pub type ApiGetMemberSelfResponse = self::GetMemberSelfResponse;

// Books
/// GET     /books
pub type ApiGetBookListResponse = self::GetBookListResponse;
/// GET     /book/{id}/posters
pub type ApiGetPosterByBookIdResponse = self::GetPostersResponse;
/// POST    /book/{id}/posters
pub type ApiPostPosterByBookIdResponse = String;
// TODO: Remove? Use /image/{type}/{id}?
/// GET     /book/{id}/thumbnail
pub type ApiGetBookThumbnailResponse = Vec<u8>;
/// GET     /book/{id}
pub type ApiGetBookByIdResponse = self::GetBookResponse;
/// POST    /book/{id}
pub type ApiPostUpdateBookResponse = ();
/// GET     /book/search
pub type ApiGetBookSearchResponse = self::BookSearchResponse;

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
pub type ApiGetIsSetupResponse = bool;
/// POST    /setup
pub type ApiPostSetupResponse = ();



// Images

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetPostersResponse {
    pub items: Vec<Poster>
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


// Libraries

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetLibrariesResponse {
    pub items: Vec<LibraryColl>
}



// Book
#[derive(Default, Debug, PartialEq, Serialize, Deserialize, Clone)]
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
    pub progress: Option<Progression>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetBookListResponse {
    pub count: usize,
    pub items: Vec<DisplayItem>
}

#[derive(Serialize, Deserialize)]
pub struct BookListQuery {
    pub library: Option<LibraryId>,
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    /// `SearchQuery`
    #[serde(default)]
    pub filters: FilterContainer,
}

impl BookListQuery {
    pub fn new(library: Option<LibraryId>, offset: Option<usize>, limit: Option<usize>, filters: FilterContainer) -> Result<Self> {
        Ok(Self {
            library,
            offset,
            limit,
            filters,
        })
    }

    pub fn has_query(&self) -> bool {
        !self.filters.filters.is_empty()
    }
}


pub type GetChaptersResponse = QueryListResponse<Chapter>;



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
    pub query: Option<String>
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModifyOptionsBody {
    pub library: Option<BasicLibrary>,
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
    AutoMatchBookIdBySource,
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
    pub items: HashMap<String, Vec<SearchItem>>
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
            _ => unreachable!()
        }
    }

    pub fn as_person(&self) -> &MetadataPersonSearchItem {
        match self {
            Self::Person(v) => v,
            _ => unreachable!()
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

    pub birth_date: Option<String>,
    pub death_date: Option<String>,
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RunTaskBody {
    #[serde(default)]
    pub run_search: bool,
    #[serde(default)]
    pub run_metadata: bool
}



#[derive(Deserialize)]
pub struct SimpleListQuery {
    pub offset: Option<usize>,
    pub limit: Option<usize>,
    pub query: Option<String>,
}