use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use crate::{MediaItem, Progression, LibraryColl, BasicLibrary, BasicDirectory, Chapter, DisplayItem, DisplayMetaItem, Person, SearchType, ThumbnailPath, Source, Member};


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
	pub count: i64,
	pub items: Vec<DisplayItem>
}

#[derive(Deserialize)]
pub struct BookListQuery {
	pub library: usize,
	pub offset: Option<usize>,
	pub limit: Option<usize>,
}



pub type GetChaptersResponse = QueryListResponse<Chapter>;



// People

pub type GetPeopleResponse = QueryListResponse<Person>;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PostPersonBody {
	AutoMatchById,

	UpdateBySource(Source),

	CombinePersonWith(i64),
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
	AutoMatchByMetaId,

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
	pub thumbnail: ThumbnailPath,
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