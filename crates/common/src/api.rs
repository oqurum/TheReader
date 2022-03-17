use serde::{Serialize, Deserialize};

use crate::{MediaItem, Progression, LibraryColl};


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

// Options

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GetOptionsResponse {
	pub libraries: Vec<LibraryColl>
}