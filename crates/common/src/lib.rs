use serde::{Serialize, Deserialize};


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaItem {
	pub id: i64,

	pub title: String,
	pub author: String,
	pub icon_path: Option<String>,

	pub chapter_count: usize,

	pub path: String,

	pub file_name: String,
	pub file_type: String,
	pub file_size: i64,

	pub modified_at: i64,
	pub accessed_at: i64,
	pub created_at: i64,
}

impl PartialEq for MediaItem {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}


#[derive(Debug, Serialize, Deserialize)]
pub struct StrippedMediaItem {
	pub id: i64,

	pub file_name: String,
	pub file_type: String,

	pub modified_at: i64,
	pub created_at: i64,
}

impl PartialEq for StrippedMediaItem {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}


#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Progression {
	Ebook {
		stopped: i32,
		total: i32
	},

	AudioBook {
		stopped: i32,
		total: i32
	}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chapter {
	pub value: usize,
	pub html: String
}