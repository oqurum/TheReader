// https://openlibrary.org/developers/api

use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Serialize, Deserialize};

use crate::database::table::{MetadataItem, File};
use super::Metadata;

pub struct OpenLibraryMetadata;

#[async_trait]
impl Metadata for OpenLibraryMetadata {
	fn get_prefix(&self) ->  & 'static str {
		"openlibrary"
	}

	async fn try_parse(&mut self, file: &File) -> Result<Option<MetadataItem>> {
		use bookie::Book;

		// Wrapped b/c "future cannot be send between threads safely"
		let found = {
			let book = bookie::epub::EpubBook::load_from_path(&file.path).unwrap();
			book.find(bookie::BookSearch::Identifier)
		};

		println!("[OL]: try_parse with ids: {:?}", found);


		if let Some(idents) = found {
			for ident in idents {
				let id = match BookId::make_assumptions(ident) {
					Some(v) => v,
					None => continue
				};

				match self.request(id).await {
					Ok(Some(v)) => return Ok(Some(v)),
					a => eprintln!("{:?}", a)
				}
			}
		}

		Ok(None)
	}
}

impl OpenLibraryMetadata {
	async fn request(&self, id: BookId) -> Result<Option<MetadataItem>> {
		let resp = reqwest::get(id.get_api_url()).await?;

		let record = resp.json::<Record>().await?;

		// TODO: Parse record.publish_date | Variations i've seen: "2018", "October 1, 1988", unknown if more types

		let source_id = match record.isbn_13.first().or_else(|| record.isbn_10.first()) {
			Some(v) => v,
			None => return Ok(None)
		};

		// TODO: Download thumb url and store it.

		Ok(Some(MetadataItem {
			id: 0,
			file_item_count: 1,
			source: format!("{}:{}", self.get_prefix(), source_id),
			title: Some(record.title.clone()),
			original_title: Some(record.title),
			description: record.description.as_ref().map(|v| v.content().to_owned()),
			rating: 0.0,
			thumb_url: None,
			creator: None,
			publisher: None,
			tags_genre: None,
			tags_collection: None,
			tags_author: None,
			tags_country: None,
			refreshed_at: Utc::now(),
			created_at: Utc::now(),
			updated_at: Utc::now(),
			deleted_at: None,
			available_at: None,
			year: None,
			hash: String::new(),
		}))
	}
}


/// https://openlibrary.org/dev/docs/api/books
pub enum BookId {
	// OpenLibrary Id
	Work(String),
	Edition(String),

	// Standard
	Isbn(String),
}

impl BookId {
	pub fn key(&self) -> &str {
		match self {
			Self::Work(_) => "works",
			Self::Edition(_) => "books",
			Self::Isbn(_) => "isbn",
		}
	}

	pub fn value(&self) -> &str {
		match self {
			Self::Work(v) => v.as_str(),
			Self::Edition(v) => v.as_str(),
			Self::Isbn(v) => v.as_str(),
		}
	}

	pub fn get_api_url(&self) -> String {
		format!("https://openlibrary.org/{}/{}.json", self.key(), self.value())
	}

	/// Tries to convert string into one of these values to the best of its' ability.
	pub fn make_assumptions(value: String) -> Option<Self> {
		match value {
			v if v.starts_with("OL") && v.ends_with('W') => Some(Self::Work(v)),
			v if v.starts_with("OL") && v.ends_with('M') => Some(Self::Edition(v)),
			v if v.chars().all(|v| ('0'..='9').contains(&v)) => Some(Self::Isbn(v)),

			_ => None
		}
	}
}


pub enum SearchId {
	Id(String),

	Isbn(String),
	Oclc(String),
	Lccn(String),
	Olid(String),

	Goodreads(String),
	LibraryThing(String)
}

impl SearchId {
	pub fn get_api_url(&self) -> String {
		format!("https://covers.openlibrary.org/b/{}/{}-L.jpg", self.key(), self.value())
	}

	pub fn key(&self) -> &str {
		match self {
			Self::Id(_) => "id",
			Self::Isbn(_) => "isbn",
			Self::Oclc(_) => "oclc",
			Self::Lccn(_) => "lccn",
			Self::Olid(_) => "olid",
			Self::Goodreads(_) => "goodreads",
			Self::LibraryThing(_) => "librarything",
		}
	}

	pub fn value(&self) -> &str {
		match self {
			Self::Id(v) => v.as_str(),
			Self::Isbn(v) => v.as_str(),
			Self::Oclc(v) => v.as_str(),
			Self::Lccn(v) => v.as_str(),
			Self::Olid(v) => v.as_str(),
			Self::Goodreads(v) => v.as_str(),
			Self::LibraryThing(v) => v.as_str()
		}
	}
}



#[derive(Serialize, Deserialize)]
pub struct Record {
	pub publishers: Vec<String>,
	pub number_of_pages: usize,
	pub description: Option<RecordDescription>,
	pub contributors: Option<Vec<Contributor>>,
	pub series: Option<Vec<String>>,
	pub covers: Vec<usize>,
	pub local_id: Option<Vec<String>>,
	pub physical_format: Option<String>,
	pub key: String,
	pub authors: Option<Vec<KeyItem>>,
	pub publish_places: Option<Vec<String>>,
	pub contributions: Option<Vec<String>>,
	pub subjects: Option<Vec<String>>,
	pub edition_name: Option<String>,
	pub pagination: Option<String>,
	// pub classifications: ,
	pub source_records: Option<Vec<String>>,
	pub title: String,
	pub identifiers: HashMap<String, Vec<String>>, // TODO: Enum Key names (amazon, google, librarything, goodreads, etc..)
	pub languages: Vec<KeyItem>,
	pub publish_date: String,
	pub copyright_date: Option<String>,
	pub works: Vec<KeyItem>,
	pub r#type: KeyItem,
	pub physical_dimensions: Option<String>,
	pub ocaid: String,
	pub isbn_10: Vec<String>,
	pub isbn_13: Vec<String>,
	pub lccn: Option<Vec<String>>,
	pub oclc_numbers: Option<Vec<String>>,
	pub lc_classifications: Option<Vec<String>>,
	pub latest_revision: usize,
	pub revision: usize,
	pub created: TypeValueItem,
	pub last_modified: TypeValueItem
}


#[derive(Serialize, Deserialize)]
#[serde(untagged)]
pub enum RecordDescription {
	Text(String),
	SpecificType(TypeValueItem)
}

impl RecordDescription {
	pub fn content(&self) -> &str {
		match self {
			Self::Text(v) => v.as_str(),
			Self::SpecificType(v) => v.value.as_str(),
		}
	}
}



#[derive(Serialize, Deserialize)]
pub struct Contributor {
	role: String,
	name: String
}

#[derive(Serialize, Deserialize)]
pub struct KeyItem {
	key: String
}

#[derive(Clone, Serialize, Deserialize)]
pub struct TypeValueItem {
	r#type: String, // TODO: Handle Types
	value: String
}

/*
Types
	/type/text = "Normal Text" (used in: description)
	/type/datetime = "2021-09-30T16:27:03.066859" (used in: create, last_modified)
*/


#[cfg(test)]
mod tests {
	use tokio::runtime::Runtime;

	use super::*;

	#[test]
	fn test_url() {
		let rt = Runtime::new().unwrap();

		rt.block_on(async {
			let resp = reqwest::get("https://openlibrary.org/books/OL7353617M.json").await.unwrap();

			resp.json::<Record>().await.unwrap();
		});
	}
}