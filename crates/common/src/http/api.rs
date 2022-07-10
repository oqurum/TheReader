use std::collections::HashMap;

use common::{ImageId, PersonId, Either, Source};
use serde::{Serialize, Deserialize};

use crate::{MediaItem, Progression, LibraryColl, BasicLibrary, BasicDirectory, Chapter, DisplayItem, DisplayMetaItem, Person, SearchType, Member, Poster, Result, LibraryId};


// API Routes

// BOOKS
/// GET     /books
pub type ApiGetBookListResponse = self::GetBookListResponse;
/// GET     /file/{id}
pub type ApiGetBookByIdResponse = Option<self::GetBookIdResponse>;
/// GET     /file/{id}/res/{tail:.*}
pub type ApiGetBookResourceByIdResponse = String;
/// GET     /file/{id}/pages/{pages}
pub type ApiGetBookPagesByIdResponse = self::GetChaptersResponse;
/// GET     /file/{id}/debug/{tail:.*}
pub type ApiGetBookDebugByIdResponse = String;
/// POST    /file/{id}/progress
pub type ApiPostBookProgressByIdResponse = ();
/// DELETE  /file/{id}/progress
pub type ApiDeleteBookProgressByIdResponse = ();
/// GET     /file/{id}/notes
pub type ApiGetBookNotesByIdResponse = Option<String>;
/// POST    /file/{id}/notes
pub type ApiPostBookNotesByIdResponse = ();
/// DELETE  /file/{id}/notes
pub type ApiDeleteBookNotesByIdResponse = ();
/// GET     /file/{id}/posters
pub type ApiGetPosterByMetaIdResponse = self::GetPostersResponse;
/// POST    /file/{id}/posters
pub type ApiPostPosterByMetaIdResponse = ();

// IMAGES
/// GET     /image/{type}/{id}
pub type ApiGetImageTypeByIdResponse = Vec<u8>;

// Libraries
/// GET     /libraries
pub type ApiGetLibrariesResponse = self::GetLibrariesResponse;

// Members
/// GET     /member
pub type ApiGetMemberSelfResponse = self::GetMemberSelfResponse;

// Metadata
// TODO: Remove? Use /image/{type}/{id}?
/// GET     /book/{id}/thumbnail
pub type ApiGetMetadataThumbnailResponse = Vec<u8>;
/// GET     /book/{id}
pub type ApiGetMetadataByIdResponse = self::MediaViewResponse;
/// POST    /book/{id}
pub type ApiPostUpdateMetadataResponse = ();
/// GET     /book/search
pub type ApiGetMetadataSearchResponse = self::MetadataSearchResponse;

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

#[derive(Default, Debug, Serialize, Deserialize, Clone)]
pub struct LoadResourceQuery {
	#[serde(default)]
	pub configure_pages: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetBookIdResponse {
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
	pub search: Option<String>,
}

impl BookListQuery {
	pub fn new(library: Option<LibraryId>, offset: Option<usize>, limit: Option<usize>, search: Option<SearchQuery>) -> Result<Self> {
		let search = search.map(serde_urlencoded::to_string)
			.transpose()?;

		Ok(Self {
			library,
			offset,
			limit,
			search,
		})
	}

	pub fn search_query(&self) -> Option<Result<SearchQuery>> {
		self.search.as_deref().map(|v| Ok(serde_urlencoded::from_str(v)?))
	}
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchQuery {
	pub query: Option<String>,
	pub source: Option<String>,
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



// Options

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetOptionsResponse {
	pub libraries: Vec<LibraryColl>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ModifyOptionsBody {
	pub library: Option<BasicLibrary>,
	pub directory: Option<BasicDirectory>
}



// Metadata

#[derive(Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct MediaViewResponse {
	pub metadata: DisplayMetaItem,
	pub media: Vec<MediaItem>,
	pub progress: Vec<Option<Progression>>,
	pub people: Vec<Person>,
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PostMetadataBody {
	AutoMatchMetaIdBySource,
	AutoMatchMetaIdByFiles,

	UpdateMetaBySource(Source)
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetMetadataSearch {
	pub query: String,
	pub search_type: SearchType,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetadataSearchResponse {
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct QueryListResponse<V> {
	pub offset: usize,
	pub limit: usize,
	pub total: usize,
	pub items: Vec<V>
}