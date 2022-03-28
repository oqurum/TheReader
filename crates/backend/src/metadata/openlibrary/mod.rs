// https://openlibrary.org/developers/api

use anyhow::Result;
use async_trait::async_trait;
use books_common::MetadataItemCached;
use chrono::Utc;
use serde::{Serialize, Deserialize};

use crate::{database::table::{MetadataItem, File, self}, ThumbnailType};
use super::{Metadata, SearchItem, MetadataReturned};

pub mod book;
pub mod author;

use book::BookId;

pub struct OpenLibraryMetadata;

#[async_trait]
impl Metadata for OpenLibraryMetadata {
	fn get_prefix(&self) ->  & 'static str {
		"openlibrary"
	}

	async fn try_parse(&mut self, file: &File) -> Result<Option<MetadataReturned>> {
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
					a => eprintln!("OpenLibraryMetadata::try_parse {:?}", a)
				}
			}
		}

		Ok(None)
	}

	async fn search(&mut self, search: &str) -> Result<Vec<SearchItem>> {
		//

		Ok(Vec::new())
	}
}

impl OpenLibraryMetadata {
	pub async fn request(&self, id: BookId) -> Result<Option<MetadataReturned>> {
		let mut book_info = if let Some(v) = book::get_book_by_id(&id).await? {
			v
		} else {
			return Ok(None);
		};


		// Find Authors.
		let authors_rfd = author::get_authors_from_book_by_rfd(&id).await?;

		// Now authors are just Vec< OL00000A >
		let authors_found = if let Some(authors) = book_info.authors.take() {
			let mut author_paths: Vec<String> = authors.into_iter()
				.map(|v| strip_url_or_path(v.key))
				.collect();

			for auth in authors_rfd {
				let stripped = strip_url_or_path(auth.about);

				if !author_paths.contains(&stripped) {
					author_paths.push(stripped);
				}
			}

			author_paths
		} else {
			authors_rfd.into_iter()
				.map(|auth| strip_url_or_path(auth.about))
				.collect()
		};

		let mut authors = Vec::new();

		// Now we'll grab the Authors.
		for auth_id in authors_found {
			println!("[OL]: Grabbing Author: {}", auth_id);

			match author::get_author_from_url(&auth_id).await {
				Ok(author) => {
					authors.push((
						table::NewTagPerson {
							source: self.prefix_text(auth_id),
							type_of: 0,
							name: author.name.clone(),
							description: author.bio.map(|v| v.into_content()),
							birth_date: author.birth_date,
							updated_at: Utc::now(),
							created_at: Utc::now(),
						},
						author.alternate_names
					));
				}

				Err(e) => eprintln!("[METADATA]: OpenLibrary Error: {}", e),
			}
		}

		// TODO: Parse record.publish_date | Millions of different variations. No specifics' were followed.

		let source_id = match book_info.isbn_13.first().or_else(|| book_info.isbn_10.as_ref().and_then(|v| v.first())) {
			Some(v) => v,
			None => return Ok(None)
		};

		let mut thumb_url = None;

		// Download thumb url and store it.
		if let Some(thumb_id) = book_info.covers.as_ref().and_then(|v| v.iter().find(|v| **v != -1)).copied() {
			let resp = reqwest::get(CoverId::Id(thumb_id.to_string()).get_api_url()).await?;

			if resp.status().is_success() {
				let bytes = resp.bytes().await?;

				match crate::store_image(ThumbnailType::Metadata, bytes.to_vec()).await {
					Ok(path) => thumb_url = Some(ThumbnailType::Metadata.prefix_text(&path)),
					Err(e) => {
						eprintln!("store_image: {}", e);
					}
				}
			}
		}

		Ok(Some(MetadataReturned {
			authors: Some(authors).filter(|v| !v.is_empty()),
			publisher: book_info.publishers.first().cloned(),

			meta: MetadataItem {
				id: 0,
				file_item_count: 1,
				source: format!("{}:{}", self.get_prefix(), source_id),
				title: Some(book_info.title.clone()),
				original_title: Some(book_info.title),
				description: book_info.description.as_ref().map(|v| v.content().to_owned()),
				rating: 0.0,
				thumb_url,
				cached: MetadataItemCached::default(),
				refreshed_at: Utc::now(),
				created_at: Utc::now(),
				updated_at: Utc::now(),
				deleted_at: None,
				available_at: None,
				year: None,
				hash: String::new(),
			}
		}))
	}
}

pub enum CoverId {
	Id(String), // TODO: number

	Isbn(String),
	Oclc(String),
	Lccn(String),
	Olid(String),

	Goodreads(String),
	LibraryThing(String)
}

impl CoverId {
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


fn strip_url_or_path<V: AsRef<str>>(value: V) -> String {
	value.as_ref()
		.rsplit('/')
		.find(|v| !v.is_empty())
		.unwrap()
		.to_owned()
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


#[derive(Debug, Serialize, Deserialize)]
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

	pub fn into_content(self) -> String {
		match self {
			Self::Text(v) => v,
			Self::SpecificType(v) => v.value,
		}
	}
}


#[cfg(test)]
mod tests {
	use tokio::runtime::Runtime;

	use super::*;

	#[test]
	fn test_json_parse_url() {
		let rt = Runtime::new().unwrap();

		rt.block_on(async {
			book::get_book_by_id(&BookId::Edition(String::from("OL7353617M"))).await.unwrap();
		});
	}
}