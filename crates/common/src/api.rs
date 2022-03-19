use serde::{Serialize, Deserialize};

use crate::{MediaItem, Progression, LibraryColl, BasicLibrary, BasicDirectory};


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

#[derive(serde::Deserialize)]
pub struct BookListQuery {
	pub offset: Option<usize>,
	pub limit: Option<usize>,
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
