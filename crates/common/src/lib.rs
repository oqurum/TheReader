use serde::{Serialize, Deserialize};


pub mod api;


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaItem {
	pub id: i64,

	pub title: String,
	pub cached: MetadataItemCached,
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
		char_pos: i64,
		page: i64,
	},

	AudioBook {
		chapter: i64,
		seek_pos: i64,
	},

	Complete
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Chapter {
	pub value: usize,
	pub html: String
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LibraryColl {
	pub id: i64,
	pub name: String,

	pub scanned_at: i64,
	pub created_at: i64,
	pub updated_at: i64,

	pub directories: Vec<String>
}


#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BasicLibrary {
	pub id: Option<i64>,
	pub name: Option<String>
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BasicDirectory {
	pub library_id: i64,
	pub path: String
}



#[derive(Debug, Default, Clone, Serialize, Deserialize)]
pub struct MetadataItemCached {
	pub author: Option<String>,
	pub publisher: Option<String>,
}

impl MetadataItemCached {
	pub fn as_string(&self) -> String {
		serde_urlencoded::to_string(&self).unwrap()
	}

	/// Returns `None` if string is empty.
	pub fn as_string_optional(&self) -> Option<String> {
		Some(self.as_string()).filter(|v| !v.is_empty())
	}

	pub fn from_string<V: AsRef<str>>(value: V) -> Self {
		serde_urlencoded::from_str(value.as_ref()).unwrap()
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
