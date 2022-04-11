use std::path::PathBuf;

use chrono::{DateTime, Utc};
use serde::{Serialize, Deserialize};

use util::*;

pub mod api;
pub mod util;
pub mod specific;

pub use specific::*;


// Used for People View

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Person {
	pub id: i64,

	pub source: Source,

	pub name: String,
	pub description: Option<String>,
	pub birth_date: Option<String>,

	pub thumb_url: ThumbnailPath,

	#[serde(serialize_with = "serialize_datetime", deserialize_with = "deserialize_datetime")]
	pub updated_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime", deserialize_with = "deserialize_datetime")]
	pub created_at: DateTime<Utc>,
}

impl Person {
	pub fn get_thumb_url(&self) -> String {
		if self.thumb_url.is_some() {
			format!("/api/person/{}/thumbnail", self.id)
		} else {
			String::from("/images/missingperson.jpg")
		}
	}
}

impl PartialEq for Person {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}


// Used for Library View

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayItem {
	pub id: i64,

	pub title: String,
	pub cached: MetadataItemCached,
	pub has_thumbnail: bool,
}

impl PartialEq for DisplayItem {
	fn eq(&self, other: &Self) -> bool {
		self.id == other.id
	}
}


// Used for Media View

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DisplayMetaItem {
	pub id: i64,

	pub library_id: i64,

	pub source: Source,
	pub file_item_count: i64,
	pub title: Option<String>,
	pub original_title: Option<String>,
	pub description: Option<String>,
	pub rating: f64,
	pub thumb_path: ThumbnailPath,

	// TODO: Make table for all tags. Include publisher in it. Remove country.
	pub cached: MetadataItemCached,

	#[serde(serialize_with = "serialize_datetime", deserialize_with = "deserialize_datetime")]
	pub refreshed_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime", deserialize_with = "deserialize_datetime")]
	pub created_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime", deserialize_with = "deserialize_datetime")]
	pub updated_at: DateTime<Utc>,
	#[serde(serialize_with = "serialize_datetime_opt", deserialize_with = "deserialize_datetime_opt")]
	pub deleted_at: Option<DateTime<Utc>>,

	pub available_at: Option<i64>,
	pub year: Option<i64>,

	pub hash: String
}

impl DisplayMetaItem {
	pub fn get_thumb_url(&self) -> String {
		if self.thumb_path.is_some() {
			format!("/api/metadata/{}/thumbnail", self.id)
		} else {
			String::from("/images/missingthumbnail.jpg")
		}
	}

	pub fn get_title(&self) -> String {
		self.title.as_ref().or(self.original_title.as_ref()).cloned().unwrap_or_else(|| String::from("No Title"))
	}
}

impl Default for DisplayMetaItem {
	fn default() -> Self {
		Self {
			id: Default::default(),
			library_id: Default::default(),
			source: Default::default(),
			file_item_count: Default::default(),
			title: Default::default(),
			original_title: Default::default(),
			description: Default::default(),
			rating: Default::default(),
			thumb_path: Default::default(),
			cached: Default::default(),
			refreshed_at: Utc::now(),
			created_at: Utc::now(),
			updated_at: Utc::now(),
			deleted_at: Default::default(),
			available_at: Default::default(),
			year: Default::default(),
			hash: Default::default()
		}
	}
}


// Used for Reader

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct MediaItem {
	pub id: i64,

	pub path: String,

	pub file_name: String,
	pub file_type: String,
	pub file_size: i64,

	pub library_id: i64,
	pub metadata_id: Option<i64>,
	pub chapter_count: usize,

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
	pub file_path: PathBuf,
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

	pub fn overwrite_with(&mut self, value: Self) {
		if value.author.is_some() {
			self.author = value.author;
		}

		if value.publisher.is_some() {
			self.publisher = value.publisher;
		}
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

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SearchType {
	Book,
	Person
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SearchFor {
	Book(SearchForBooksBy),
	Person,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum SearchForBooksBy {
	Query,
	Title,
	AuthorName,
	Contents,
}