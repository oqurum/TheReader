use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use crate::{MediaItem, Progression, LibraryColl, BasicLibrary, BasicDirectory, Chapter};


// Libraries

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetLibrariesResponse {
	pub items: Vec<LibraryColl>
}



// Book

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetBookIdResponse {
	pub media: MediaItem,
	pub progress: Option<Progression>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetBookListResponse {
	pub count: i64,
	pub items: Vec<MediaItem>
}

#[derive(Deserialize)]
pub struct BookListQuery {
	pub library: usize,
	pub offset: Option<usize>,
	pub limit: Option<usize>,
}



#[derive(Serialize, Deserialize)]
pub struct GetChaptersResponse {
	pub offset: usize,
	pub limit: usize,
	pub total: usize,
	pub chapters: Vec<Chapter>
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

#[derive(Debug, Serialize, Deserialize, Clone)]
pub enum PostMetadataBody {
	File(i64)
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetMetadataSearch {
	pub query: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetadataSearchResponse {
	pub items: HashMap<String, Vec<MetadataSearchItem>>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MetadataSearchItem {
	pub thumbnail: Option<String>,
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