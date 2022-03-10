use serde::{Serialize, Deserialize};


pub mod api;


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


#[derive(Debug, Serialize, Deserialize, Clone, Copy)]
pub enum Progression {
	Ebook {
		chapter: i64,
		page: i64,
	},

	AudioBook {
		chapter: i64,
		seek_pos: i64,
	}
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chapter {
	pub value: usize,
	pub html: String
}