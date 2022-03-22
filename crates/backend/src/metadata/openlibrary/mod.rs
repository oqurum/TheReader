// https://openlibrary.org/developers/api

// TODO: Handle Author

use anyhow::Result;
use async_trait::async_trait;
use chrono::Utc;
use serde::{Serialize, Deserialize};

use crate::database::table::{MetadataItem, File};
use super::Metadata;

pub mod book;
pub mod author;

use book::{BookId, Record};

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
		let resp = reqwest::get(id.get_json_url()).await?;

		let record = resp.json::<Record>().await?;

		// TODO: Parse record.publish_date | Variations i've seen: "2018", "October 1, 1988", unknown if more types

		let source_id = match record.isbn_13.first().or_else(|| record.isbn_10.first()) {
			Some(v) => v,
			None => return Ok(None)
		};

		println!("{:#?}", record);

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

/*
Types
	/type/text = "Normal Text" (used in: description)
	/type/datetime = "2021-09-30T16:27:03.066859" (used in: create, last_modified)
*/


#[derive(Debug, Serialize, Deserialize)]
pub struct KeyItem {
	key: String
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeValueItem {
	r#type: String, // TODO: Handle Types
	value: String
}


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