use std::collections::HashMap;

use anyhow::Result;
use async_trait::async_trait;

use crate::database::{table::{MetadataItem, File, self}, Database};

use self::{
	google_books::GoogleBooksMetadata,
	local::LocalMetadata,
	openlibrary::OpenLibraryMetadata
};

pub mod audible;
pub mod commonsensemedia;
pub mod google_books;
pub mod goodreads;
pub mod local;
pub mod openlibrary;
pub mod ratedreads;

// "source" column: [prefix]:[id]

/// Simple return if found, println if error.
macro_rules! return_if_found {
	($v: expr) => {
		match $v {
			Ok(Some(v)) => return Ok(Some(v)),
			Ok(None) => (),
			Err(e) => eprintln!("metadata::get_metadata: {}", e)
		}
	};
}


#[async_trait]
pub trait Metadata {
	fn prefix_text<V: AsRef<str>>(&self, value: V) -> String {
		format!("{}:{}", self.get_prefix(), value.as_ref())
	}

	fn get_prefix(&self) -> &'static str;

	async fn get_metadata_from_file(&mut self, file: &File) -> Result<Option<MetadataReturned>>;

	async fn search(&mut self, _search: &str, search_for: SearchFor) -> Result<Vec<SearchItem>> {
		Ok(Vec::new())
	}
}

// TODO: Utilize current metadata in get_metadata_from_file.
// TODO: Order which metadata should be tried.
pub async fn get_metadata(file: &File, _meta: Option<&MetadataItem>, db: &Database) -> Result<Option<MetadataReturned>> {
	return_if_found!(OpenLibraryMetadata.get_metadata_from_file(file).await);
	return_if_found!(GoogleBooksMetadata.get_metadata_from_file(file).await);

	// TODO: Don't re-scan file if we already have metadata from file.
	LocalMetadata.get_metadata_from_file(file).await
}


pub async fn search_all_agents(search: &str, search_for: SearchFor) -> Result<HashMap<&'static str, Vec<SearchItem>>> {
	let mut map = HashMap::new();

	map.insert(
		OpenLibraryMetadata.get_prefix(),
		OpenLibraryMetadata.search(search, search_for).await?,
	);

	map.insert(
		GoogleBooksMetadata.get_prefix(),
		GoogleBooksMetadata.search(search, search_for).await?,
	);

	Ok(map)
}


#[derive(Debug, Clone, Copy)]
pub enum SearchFor {
	Book, // TODO: Allow specifics. Ex: Regular Query, Title, Author Name, Contents
	Author,
}


#[derive(Debug)]
pub enum SearchItem {
	Author(AuthorSearchInfo),
	Book(MetadataItem)
}

// TODO: Replace MetadataReturned.authors with this.
#[derive(Debug)]
pub struct AuthorSearchInfo {
	pub source: String,

	pub cover_image: Option<String>,

	pub name: String,
	pub other_names: Option<Vec<String>>,
	pub description: Option<String>,

	pub birth_date: Option<String>,
	pub death_date: Option<String>,
}


#[derive(Debug)]
pub struct MetadataReturned {
	// Person, Alt Names
	pub authors: Option<Vec<(table::NewTagPerson, Option<Vec<String>>)>>,
	pub publisher: Option<String>,
	// TODO: Add More.

	pub meta: MetadataItem
}

impl MetadataReturned {
	/// Returns (Main Author, Person IDs)
	pub fn add_or_ignore_authors_into_database(&mut self, db: &Database) -> Result<(Option<String>, Vec<i64>)> {
		let mut main_author = None;
		let mut person_ids = Vec::new();

		if let Some(authors_with_alts) = self.authors.take() {
			for (author, other_names) in authors_with_alts {
				// Check if we already have a person by that name anywhere in the two database tables.
				if let Some(person) = db.get_person_by_name(&author.name)? {
					person_ids.push(person.id);

					if main_author.is_none() {
						main_author = Some(person.name);
					}

					continue;
				}

				let person_id = db.add_person(&author)?;

				if let Some(alts) = other_names {
					for name in alts {
						// Ignore errors. Errors should just be UNIQUE constraint failed
						if let Err(e) = db.add_person_alt(&table::TagPersonAlt {
							person_id,
							name,
						}) {
							eprintln!("[OL]: Add Alt Name Error: {e}");
						}
					}
				}

				person_ids.push(person_id);

				if main_author.is_none() {
					main_author = Some(author.name);
				}
			}
		}

		Ok((main_author, person_ids))
	}
}